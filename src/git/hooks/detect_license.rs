use crate::git::utils::{read_blob_content, repo_files_at_head};
use crate::licenses::{license_file_names, LICENSE_STORE};
use crate::repository::Repository;

use anyhow::Result;
use askalono::TextData;
use bstr::ByteSlice;
use git_repository::objs::tree::EntryMode;
use git_repository::odb::pack::cache::DecodeEntry;
use tracing::instrument;

#[instrument(err, skip(gitoxide_repo, cache))]
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
                let content = read_blob_content(gitoxide_repo, entry.oid, cache).await?;

                detect_license_from_file(repo, content.as_str()).await;
                break;
            }
            EntryMode::Link => { /* todo: follow symlinks in case the target is a license */ }
            _ => { /* ignore directories, symlinks and submodules */ }
        }
    }

    Ok(())
}

#[instrument]
async fn detect_license_from_file(repo: &mut Repository, data: &str) {
    let text_data = TextData::from(data);

    let license_store = LICENSE_STORE.lock().await;
    let license_match = license_store.analyze(&text_data);

    // Only apply license if we're confident
    repo.license = if license_match.score >= 0.9 {
        Some(license_match.name.to_owned())
    } else {
        None
    };
}
