use crate::error::GAErrors::HookError;
use crate::git::utils::repo_files_at_head;
use crate::LICENSE_STORE;
use crate::licenses::license_file_names;
use crate::repository::Repository;

use anyhow::{Context, Result};
use askalono::TextData;
use bstr::ByteSlice;
use git_object::tree::EntryMode;
use git_odb::FindExt;
use git_pack::cache::DecodeEntry;

pub(crate) async fn detect_license(repo: &mut Repository, gitoxide_repo: &git_repository::Repository, cache: &mut impl DecodeEntry) -> Result<()> {
    let mut buffer = Vec::<u8>::new();

    let tree = repo_files_at_head(gitoxide_repo, &mut buffer, cache).await?;

    for entry in tree.entries {
        let lowered_file_name = entry.filename.to_lowercase();

        if !license_file_names().contains(&lowered_file_name.as_slice()) {
            continue
        }

        match entry.mode {
            EntryMode::Blob => {
                let mut blob_buffer = Vec::<u8>::new();
                let blob = gitoxide_repo.odb.find_existing_blob(entry.oid, &mut blob_buffer, cache)?;

                // Honestly no idea how but this works out to yield valid file content
                // TODO: Maybe Git odb has some header and padding attached to the blob? Need to investigate
                let content_vec: Vec<u8> = blob.data.iter()
                    .map(|i| *i)
                    .skip(2)
                    .filter(|b| *b != 0)
                    .collect();

                let content = &content_vec[..content_vec.len() - 2];

                detect_license_from_file(repo, content).await?;
                break;
            }
            EntryMode::Link => { /* todo: follow symlinks in case the target is a license */ }
            _ => { /* ignore directories, symlinks and submodules */ }
        }
    }

    Ok(())
}

async fn detect_license_from_file(repo: &mut Repository, data: &[u8]) -> Result<()> {
    let file_content = String::from_utf8(data.to_vec()).context("Failed to decode license file content into valid UTF-8")?;
    let text_data = TextData::from(file_content);

    let license_store = LICENSE_STORE.lock().map_err(|_| HookError("Failed to acquire lock for license store"))?;
    let license_match = license_store.analyze(&text_data);

    // Only apply license if we're confident
    repo.license = if license_match.score >= 0.9 {
        Some(license_match.name.to_owned())
    } else {
        None
    };

    Ok(())
}
