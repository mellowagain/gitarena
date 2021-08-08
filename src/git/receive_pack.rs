use crate::error::GAErrors::{GitError, PackUnpackError};
use crate::extensions::{default_signature, gitoxide_to_libgit2_type, str_to_oid};
use crate::git::ref_update::RefUpdate;
use crate::git::writer::GitWriter;
use crate::extensions::traits::GitoxideSignatureExtension;
use crate::repository::Repository;

use std::convert::TryInto;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use bstr::BString;
use git2::{BranchType, Oid, Repository as Git2Repository};
use git_hash::oid;
use git_lock::acquire::Fail;
use git_object::immutable::Commit as ImmutableCommit;
use git_object::Kind;
use git_odb::pack::cache;
use git_pack::cache::lru::MemoryCappedHashmap;
use git_pack::data::{File as DataFile, ResolvedBase};
use git_pack::index::File as IndexFile;
use git_ref::mutable::Target;
use git_ref::transaction::{Change, Create, LogChange, RefEdit, RefLog};
use git_repository::prelude::*;
use hex::FromHex;

pub(crate) async fn process_create(ref_update: &RefUpdate, repo: &Git2Repository, writer: &mut GitWriter, index_file: &IndexFile, data_file: &DataFile, cache: MemoryCappedHashmap) -> Result<MemoryCappedHashmap> {
    todo!();

    let new_oid = ref_update.new.as_ref().unwrap();
    let new_oid_bytes = Vec::from_hex(new_oid)?;

    let libgit2_oid = Oid::from_str(new_oid.as_str())?;
    let gitoxide_oid = oid::try_from(new_oid_bytes.as_slice())?;

    let index = index_file.lookup(gitoxide_oid).ok_or(PackUnpackError("oid offset"))?;
    let pack_offset = index_file.pack_offset_at_index(index);

    let entry = data_file.entry(pack_offset);

    // Convert from gitoxide header type to libgit2 object type
    let ref_type = gitoxide_to_libgit2_type(&entry.header)?;

    let mut output = vec![0_u8; entry.decompressed_size as usize];

    data_file.decompress_entry(&entry, output.as_mut_slice())?;


    let _head = repo.head()?;

    let _odb = repo.odb()?;

    repo.odb()?.write(ref_type, output.as_slice())?;

    match repo.find_reference(ref_update.target_ref.as_str()) {
        Ok(mut reference) => {
            reference.set_target(libgit2_oid, "gitarena")?;
        },
        Err(_) => {
            let _a = repo.find_branch("s", BranchType::Local)?;

            //repo.worktrees()?.

            //repo.reference_symbolic_matching()

            // Reference does not exist, create new
            //repo.find_reference() // TODO: Check how git handles everything (this, and the .set_target above)
        }
    }

    let mut temp_writer = GitWriter::new();
    temp_writer.write_text(format!("ok {}", &ref_update.target_ref)).await?;
    let bytes = temp_writer.serialize().await?;

    let concatted = [b"\x01", &bytes[..]].concat();

    writer.write_binary(&concatted).await?;

    Ok(cache)
}

pub(crate) async fn process_delete(ref_update: &RefUpdate, repo: &Repository, repo_owner: &str, writer: &mut GitWriter) -> Result<()> {
    assert!(ref_update.old.is_some());
    assert!(ref_update.new.is_none());

    let gitoxide_repo = repo.gitoxide(repo_owner).await?;

    let object_id = str_to_oid(&ref_update.old).await
        .map_err(|_| GitError(404, Some("Ref does not exist".to_owned())))?;

    let edits = vec![
        RefEdit {
            change: Change::Delete {
                previous: Some(Target::Peeled(object_id)),
                log: RefLog::AndReference
            },
            name: ref_update.target_ref.as_str().try_into()?,
            deref: true
        }
    ];

    gitoxide_repo.refs.transaction()
        .prepare(edits, Fail::Immediately)
        .map_err(|e| GitError(500, Some(format!("Failed to commit transaction: {}", e))))?
        .commit(&default_signature())?;

    if ref_update.report_status || ref_update.report_status_v2 {
        // Hacky way to write the text to band 1 as GitWriter does not yet support sidebands
        // TODO: Replace this ugly code with sideband support in GitWriter
        writer.write_binary(&{
            let mut temp_writer = GitWriter::new();
            temp_writer.write_text(format!("ok {}", ref_update.target_ref)).await?;
            let bytes = temp_writer.serialize().await?;
            [b"\x01", &bytes[..]].concat()
        }).await?;
    }

    Ok(())
}

pub(crate) async fn process_update(ref_update: &RefUpdate, repo: &Repository, repo_owner: &str, writer: &mut GitWriter, index_path: &PathBuf, pack_path: &PathBuf, raw_pack: &[u8], cache: MemoryCappedHashmap) -> Result<MemoryCappedHashmap> {
    assert!(ref_update.old.is_some());
    assert!(ref_update.new.is_some());

    let mut mut_cache = cache;

    let old_oid = str_to_oid(&ref_update.old).await?;
    let new_oid = str_to_oid(&ref_update.new).await?;

    // # Gitoxide zone
    // This block decodes the entry from the pack file, creates a Gitoxide Commit and then writes it to the reflog using a transaction
    {
        let gitoxide_repo = repo.gitoxide(repo_owner).await?;
        let index_file = IndexFile::at(index_path)?;

        let index = index_file.lookup(new_oid.as_ref()).ok_or(PackUnpackError("oid index"))?;
        let offset = index_file.pack_offset_at_index(index);

        let data_file = DataFile::at(pack_path)?;

        let entry = data_file.entry(offset);
        let mut out = Vec::<u8>::with_capacity(entry.decompressed_size as usize);

        let outcome = data_file.decode_entry(
            entry,
            &mut out,
            |oid, vec| {
                if let Some(index) = index_file.lookup(oid) {
                    let offset = index_file.pack_offset_at_index(index);
                    let entry = data_file.entry(offset);

                    Some(ResolvedBase::InPack(entry))
                } else {
                    match gitoxide_repo.odb.find(oid, vec, &mut cache::Never) {
                        Ok(Some(object)) => {
                            Some(ResolvedBase::OutOfPack {
                                kind: object.kind,
                                end: vec.len()
                            })
                        }
                        Ok(None) => None,
                        Err(_) => None,
                    }
                }
            },
            &mut mut_cache
        )?;

        let commit = match outcome.kind {
            Kind::Commit => {
                ImmutableCommit::from_bytes(out.as_slice())?
            }
            _ => return Err(GitError(400, Some("Unexpected payload data type".to_owned())).into())
        };

        let edits = vec![
            RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: true,
                        message: BString::from(commit.message)
                    },
                    mode: Create::OrUpdate {
                        previous: Some(Target::Peeled(old_oid))
                    },
                    new: Target::Peeled(new_oid)
                },
                name: ref_update.target_ref.as_str().try_into()?,
                deref: true
            }
        ];

        gitoxide_repo.refs.transaction()
            .prepare(edits, Fail::Immediately)
            .map_err(|e| GitError(500, Some(format!("Failed to commit transaction: {}", e))))?
            .commit(&commit.committer.to_mut())?;
    }

    // # libgit2 zone
    // This block writes the payload into the repo odb
    {
        let git2_repo = repo.libgit2(repo_owner).await?;

        let odb = git2_repo.odb()?;
        let mut pack_writer = odb.packwriter()?;

        pack_writer.write(raw_pack)?;
        pack_writer.commit()?;
    }

    // TODO: Run `git gc --auto --quiet` to optimize repo size

    if ref_update.report_status || ref_update.report_status_v2 {
        // Hacky way to write the text to band 1 as GitWriter does not yet support sidebands
        // TODO: Replace this ugly code with sideband support in GitWriter
        writer.write_binary(&{
            let mut temp_writer = GitWriter::new();
            temp_writer.write_text(format!("ok {}", ref_update.target_ref)).await?;
            let bytes = temp_writer.serialize().await?;
            [b"\x01", &bytes[..]].concat()
        }).await?;
    }

    Ok(mut_cache)
}
