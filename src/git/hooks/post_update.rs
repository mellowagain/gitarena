use crate::git::hooks::detect_license::detect_license;
use crate::repository::Repository;

use anyhow::Result;
use git_odb::pack::cache::lru::MemoryCappedHashmap;
use log::warn;

// TODO: run these async in the background without waiting
// prefered: https://www.reddit.com/r/rust/comments/fddf6y/handling_longrunning_background_tasks_in_actixweb/
// https://stackoverflow.com/a/66181410

pub(crate) async fn run(repo: &mut Repository, owner_username: &str, cache: MemoryCappedHashmap) -> Result<MemoryCappedHashmap> {
    let gitoxide_repo = repo.gitoxide(owner_username).await?;
    let mut mut_cache = cache;

    if let Err(e) = detect_license(repo, &gitoxide_repo, &mut mut_cache).await {
        warn!("Failed to detect license for repo id {}: {}", repo.id, e);
    }

    Ok(mut_cache)
}
