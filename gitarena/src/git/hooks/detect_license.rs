use crate::git::utils::{read_blob_content, repo_files_at_head};
use crate::licenses;
use crate::licenses::license_file_names;
use crate::repository::Repository;

use std::sync::Arc;

use anyhow::Result;
use askalono::TextData;
use bstr::ByteSlice;
use gix::objs::tree::EntryKind;
use gix::odb::Store;
use tracing::instrument;

#[instrument(err, skip(store))]
pub(crate) async fn detect_license(store: Arc<Store>, gitoxide_repo: &gix::Repository, repo: &mut Repository) -> Result<()> {
    let mut buffer = Vec::<u8>::new();

    let tree = repo_files_at_head(store.clone(), gitoxide_repo, &mut buffer).await?;

    for entry in tree.entries {
        let lowered_file_name = entry.filename.to_lowercase();

        if !license_file_names().contains(&lowered_file_name.as_slice()) {
            continue;
        }

        #[allow(clippy::match_same_arms)]
        match entry.mode.kind() {
            EntryKind::Blob | EntryKind::BlobExecutable => {
                let content = read_blob_content(entry.oid, store).await?;

                detect_license_from_file(repo, content.as_str()).await;
                break;
            }
            EntryKind::Link => { /* todo: follow symlinks in case the target is a license */ }
            EntryKind::Tree => {}
            EntryKind::Commit => { /* ignore directories and submodules */ }
        }
    }

    Ok(())
}

#[instrument]
async fn detect_license_from_file(repo: &mut Repository, data: &str) {
    let text_data = TextData::from(data);

    let license_match = licenses::store().analyze(&text_data);

    // Only apply license if we're confident
    repo.license = if license_match.score >= 0.9 {
        Some(license_match.name.to_owned())
    } else {
        None
    };
}
