use crate::error::GAErrors::{GitError, PackUnpackError};
use crate::extensions::gitoxide_to_libgit2_type;
use crate::git::ref_update::RefUpdate;
use crate::git::writer::GitWriter;

use anyhow::Result;
use git2::{BranchType, Oid, Repository as Git2Repository};
use git_hash::oid;
use git_pack::data::File as DataFile;
use git_pack::index::File as IndexFile;
use hex::FromHex;

pub(crate) async fn process_create(ref_update: &RefUpdate, repo: &Git2Repository, writer: &mut GitWriter, index_file: &IndexFile, data_file: &DataFile) -> Result<()> {
    unimplemented!();

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

    Ok(())
}

pub(crate) async fn process_delete(ref_update: &RefUpdate, repo: &Git2Repository, writer: &mut GitWriter) -> Result<()> {
    assert!(ref_update.old.is_some());
    assert!(ref_update.new.is_none());

    let mut reference = repo.find_reference(ref_update.target_ref.as_str())
        .map_err(|_| GitError(404, Some("Ref does not exist".to_owned())))?;

    /*if let Some(target) = reference.target() {
        if ref_update.old.unwrap() != format!("{}", target) {
            return Err(GitError(401, Some("Tried to delete ref pointing to a wrong commit, `git pull` first".to_owned())).into());
        }
    }*/

    // This should never error as the repository is currently locked by a transaction
    reference.delete()?;

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

pub(crate) async fn process_update(_ref_update: &RefUpdate, _repo: &Git2Repository, _writer: &mut GitWriter, _index_file: &IndexFile, _data_file: &DataFile) -> Result<()> {
    unimplemented!();
}
