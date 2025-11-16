use crate::git::hooks::detect_license::detect_license;
use crate::repository::Repository;

use std::sync::Arc;

use anyhow::Result;
use git_repository::odb::Store;
use gitarena_common::database::Database;
use log::warn;
use sqlx::Transaction;

// TODO: run these async in the background without waiting
// prefered: https://www.reddit.com/r/rust/comments/fddf6y/handling_longrunning_background_tasks_in_actixweb/
// https://stackoverflow.com/a/66181410

pub(crate) async fn run(
    store: Arc<Store>,
    repo: &mut Repository,
    tx: &mut Transaction<'_, Database>,
) -> Result<()> {
    let gitoxide_repo = repo.gitoxide(tx).await?;

    if let Err(err) = detect_license(store, &gitoxide_repo, repo).await {
        warn!("Failed to detect license for repo id {}: {}", repo.id, err);
    }

    Ok(())
}
