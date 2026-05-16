use crate::config::get_setting;
use crate::database::Database;
use crate::queue::GLOBAL_QUEUE;
use crate::repository::Repository;
use crate::zoekt::task::ZoektIndexRepo;
use anyhow::{Result, anyhow};
use fang::{AsyncQueueable, AsyncRunnable};
use sqlx::Transaction;
use tracing::{debug, instrument};

#[instrument(err, skip(tx))]
pub(crate) async fn schedule_repo_indexing(repo: Repository, tx: &mut Transaction<'_, Database>) -> Result<()> {
    if !get_setting::<bool>("zoekt.enabled", tx).await? {
        return Ok(());
    }

    let task = ZoektIndexRepo { repo };

    let queued_task = GLOBAL_QUEUE
        .get()
        .ok_or_else(|| anyhow!("repo should only be attempted to be indexed after queue has been initialized"))?
        .insert_task(&task as &dyn AsyncRunnable)
        .await?;

    debug!(task.id = %queued_task.id, "queued repo to be indexed");
    Ok(())
}
