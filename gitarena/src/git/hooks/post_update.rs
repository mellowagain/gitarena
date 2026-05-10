use crate::git::hooks::detect_languages::detect_languages;
use crate::git::hooks::detect_license::detect_license;
use crate::repository::Repository;

use std::sync::Arc;

use anyhow::Result;
use gitarena_common::database::Database;
use gix::odb::Store;
use sqlx::Transaction;
use tracing::{instrument, warn};

// TODO: run these async in the background without waiting
// prefered: https://www.reddit.com/r/rust/comments/fddf6y/handling_longrunning_background_tasks_in_actixweb/
// https://stackoverflow.com/a/66181410

#[instrument(err, skip(store, tx))]
pub(crate) async fn run(store: Arc<Store>, repo: &mut Repository, tx: &mut Transaction<'_, Database>) -> Result<()> {
    let gitoxide_repo = repo.gitoxide(tx).await?;

    if let Err(err) = detect_license(store.clone(), &gitoxide_repo, repo).await {
        warn!(repo.id = %repo.id, ?err, "Failed to detect license in repo");
    }

    if let Err(err) = detect_languages(store, &gitoxide_repo, repo).await {
        warn!(repo.id = %repo.id, ?err, "Failed to detect languages in repo");
    }

    Ok(())
}
