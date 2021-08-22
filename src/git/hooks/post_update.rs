use crate::error::GAErrors::HookError;
use crate::LICENSE_STORE;
use crate::licenses::license_file_names;
use crate::repository::Repository;

use anyhow::{Context, Result};
use askalono::TextData;
use async_recursion::async_recursion;
use bstr::ByteSlice;
use git_odb::FindExt;
use git_odb::pack::cache::lru::MemoryCappedHashmap;
use git_ref::file::loose::Reference;
use git_ref::mutable::Target;

pub(crate) async fn run(repo: &mut Repository, owner_username: &str, cache: MemoryCappedHashmap) -> Result<MemoryCappedHashmap> {
    let gitoxide_repo = repo.gitoxide(owner_username).await?;
    let mut mut_cache = cache;

    // All hooks that require HEAD to point to something
    match gitoxide_repo.refs.loose_find("HEAD")? {
        Some(reference) => {
            mut_cache = detect_license(repo, &gitoxide_repo, reference, mut_cache).await?;
        }
        None => {}
    }

    Ok(mut_cache)
}

#[async_recursion]
pub(crate) async fn detect_license(repo: &mut Repository, gitoxide_repo: &git_repository::Repository, reference: Reference, cache: MemoryCappedHashmap) -> Result<MemoryCappedHashmap> {
    match reference.target {
        Target::Peeled(object_id) => {
            let mut mut_cache = cache;
            let mut buffer = Vec::<u8>::new();

            let head_tree_oid = {
                let mut buffer = Vec::<u8>::new();

                let commit = gitoxide_repo.odb.find_existing_commit(object_id.as_ref(), &mut buffer, &mut mut_cache)
                    .context("Unable to find commit pointed to by HEAD")?;

                commit.tree()
            };

            let tree = gitoxide_repo.odb.find_existing_tree(head_tree_oid.as_ref(), &mut buffer, &mut mut_cache)
                .context("Unable to find tree associated with commit pointed to by HEAD")?;

            let mut found_file = false;

            'outer: for entry in tree.entries {
                let lowered = entry.filename.to_lowercase();

                for file_name in license_file_names() {
                    if lowered.starts_with(file_name) {
                        found_file = true;

                        let mut buffer = Vec::<u8>::new();

                        let blob = gitoxide_repo.odb.find_existing_blob(entry.oid, &mut buffer, &mut mut_cache)
                            .context("Unable to find blob of license file")?;

                        let content_vec: Vec<u8> = blob.data.iter()
                            .map(|i| *i)
                            .skip(2)
                            .filter(|b| *b != 0)
                            .collect();

                        let content = &content_vec[..content_vec.len() - 2];

                        detect_license_from_file(repo, content).await?;
                        break 'outer;
                    }
                }
            }

            // The repo license file was not found (most likely deleted), set license to None
            if !found_file {
                repo.license = None;
            }

            Ok(mut_cache)
        }
        Target::Symbolic(target) => match gitoxide_repo.refs.loose_find(target.to_partial())? {
            Some(reference) => detect_license(repo, gitoxide_repo, reference, cache).await,
            None => Err(HookError("Repo symlink points to invalid target").into())
        }
    }
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
