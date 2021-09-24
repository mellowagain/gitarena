use crate::error::GAErrors::{GitError, PackUnpackError};
use crate::extensions::{default_signature, str_to_oid};
use crate::git::io::band::Band;
use crate::git::io::writer::GitWriter;
use crate::git::ref_update::RefUpdate;
use crate::repository::Repository;

use std::convert::TryInto;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use bstr::BString;
use git_lock::acquire::Fail;
use git_object::{CommitRef, Kind};
use git_odb::pack::cache;
use git_pack::cache::lru::MemoryCappedHashmap;
use git_pack::data::{File as DataFile, ResolvedBase};
use git_pack::index::File as IndexFile;
use git_ref::Target;
use git_ref::transaction::{Change, Create, LogChange, RefEdit, RefLog};
use git_repository::actor::Signature;
use git_repository::prelude::{Find, FindExt};
use tracing::instrument;
use tracing_unwrap::ResultExt;

#[instrument(err, skip(writer, cache))]
pub(crate) async fn process_create_update(ref_update: &RefUpdate, repo: &Repository, repo_owner: &str, writer: &mut GitWriter, index_path: Option<&PathBuf>, pack_path: Option<&PathBuf>, raw_pack: &[u8], cache: MemoryCappedHashmap) -> Result<MemoryCappedHashmap> {
    assert!(ref_update.new.is_some());

    let mut mut_cache = cache;
    let new_oid = str_to_oid(&ref_update.new)?;

    // # Gitoxide zone
    // This block decodes the entry from the pack file, creates a Gitoxide Commit and then writes it to the reflog using a transaction
    {
        let gitoxide_repo = repo.gitoxide(repo_owner).await?;
        let mut buffer = Vec::<u8>::new();

        let commit = match (index_path, pack_path) {
            (Some(index_path), Some(pack_path)) => {
                let index_file = IndexFile::at(index_path)?;

                let index = index_file.lookup(new_oid.as_ref()).ok_or(PackUnpackError("oid index"))?;
                let offset = index_file.pack_offset_at_index(index);

                let data_file = DataFile::at(pack_path)?;

                let entry = data_file.entry(offset);

                buffer.reserve(entry.decompressed_size as usize);

                let outcome = data_file.decode_entry(
                    entry,
                    &mut buffer,
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

                match outcome.kind {
                    Kind::Commit => CommitRef::from_bytes(buffer.as_slice())?,
                    _ => return Err(GitError(400, Some("Unexpected payload data type".to_owned())).into())
                }
            },
            _ => {
                // This is a force push to an existing repository. TODO: Handle non existing refs as client errors instead of server errors
                gitoxide_repo.odb.find_existing_commit(new_oid.as_ref(), &mut buffer, &mut mut_cache)?
            }
        };

        let previous = ref_update.old.as_ref().map(|target| Target::Peeled(str_to_oid(&Some(target.to_owned())).unwrap_or_log()));

        let edits = vec![
            RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: true,
                        message: BString::from(commit.message)
                    },
                    mode: Create::OrUpdate {
                        previous
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
            .commit(&Signature::from(commit.committer))?;
    }

    // # libgit2 zone
    // This block writes the payload into the repo odb
    {
        let git2_repo = repo.libgit2(repo_owner).await?;

        let odb = git2_repo.odb()?;
        let mut pack_writer = odb.packwriter()?;

        pack_writer.write_all(raw_pack)?;
        pack_writer.commit()?;
    }

    if ref_update.report_status || ref_update.report_status_v2 {
        writer.write_text_sideband_pktline(Band::Data, format!("ok {}", ref_update.target_ref)).await?;
    }

    Ok(mut_cache)
}

#[instrument(err, skip(writer))]
pub(crate) async fn process_delete(ref_update: &RefUpdate, repo: &Repository, repo_owner: &str, writer: &mut GitWriter) -> Result<()> {
    assert!(ref_update.old.is_some());
    assert!(ref_update.new.is_none());

    let gitoxide_repo = repo.gitoxide(repo_owner).await?;

    let object_id = str_to_oid(&ref_update.old).map_err(|_| GitError(404, Some("Ref does not exist".to_owned())))?;

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
        writer.write_text_sideband_pktline(Band::Data, format!("ok {}", ref_update.target_ref)).await?;
    }

    Ok(())
}
