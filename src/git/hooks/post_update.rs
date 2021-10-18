use crate::git::GitoxideCacheList;
use crate::git::hooks::detect_license::detect_license;
use crate::repository::Repository;

use anyhow::Result;
use log::warn;
use sqlx::{Executor, Postgres};

// TODO: run these async in the background without waiting
// prefered: https://www.reddit.com/r/rust/comments/fddf6y/handling_longrunning_background_tasks_in_actixweb/
// https://stackoverflow.com/a/66181410

pub(crate) async fn run<'e, E: Executor<'e, Database = Postgres>>(repo: &mut Repository, executor: E, cache: GitoxideCacheList) -> Result<GitoxideCacheList> {
    let gitoxide_repo = repo.gitoxide(executor).await?;
    let mut mut_cache = cache;

    if let Err(e) = detect_license(repo, &gitoxide_repo, &mut mut_cache).await {
        warn!("Failed to detect license for repo id {}: {}", repo.id, e);
    }

    Ok(mut_cache)
}
