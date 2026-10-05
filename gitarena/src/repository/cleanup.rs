use crate::config::get_setting;
use crate::database::Database;
use crate::meili::{ISSUES_MEILI_INDEX, MeiliClient, REPOS_MEILI_INDEX};
use crate::queue::GLOBAL_QUEUE;
use crate::release::assets::ReleaseAssets;
use crate::repository::task::DeletedRepoCleanupTask;
use anyhow::{Result, anyhow, bail};
use fang::{AsyncQueueable, AsyncRunnable};
use sqlx::Transaction;
use std::io::ErrorKind;
use tokio::fs;
use tracing::{error, instrument};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct RepoCleanup {
    repo_ids: Vec<Uuid>,
    issue_ids: Vec<Uuid>,
    s3_keys: Vec<String>,
    moved: Option<(String, String)>,
}

impl RepoCleanup {
    #[instrument(skip(tx))]
    pub(crate) async fn prepare(repo_ids: Vec<Uuid>, tx: &mut Transaction<'_, Database>) -> Result<Self> {
        let assets: Vec<ReleaseAssets> = sqlx::query_as("select ra.* from release_assets ra join releases r on r.id = ra.release_id where r.repo_id = any($1)")
            .bind(&repo_ids)
            .fetch_all(&mut **tx)
            .await?;

        let issue_ids: Vec<Uuid> = sqlx::query_scalar("select id from issue_cache where repo_id = any($1)")
            .bind(&repo_ids)
            .fetch_all(&mut **tx)
            .await?;

        sqlx::query("delete from events where subject_id_repo = any($1) and class = 'activity'")
            .bind(&repo_ids)
            .execute(&mut **tx)
            .await?;

        Ok(Self {
            repo_ids,
            issue_ids,
            s3_keys: assets.iter().map(|asset| asset.s3_key().to_string()).collect(),
            moved: None,
        })
    }

    #[instrument(skip(tx))]
    pub(crate) async fn move_to_trash(&mut self, path: String, id: Uuid, tx: &mut Transaction<'_, Database>) -> Result<()> {
        let base_dir: String = get_setting("repositories.base_dir", tx).await?;
        let trash_dir = format!("{base_dir}/.deleted");
        let trash_path = format!("{trash_dir}/{id}");

        fs::create_dir_all(&trash_dir).await?;

        match fs::rename(&path, &trash_path).await {
            Ok(()) => self.moved = Some((path, trash_path)),
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => bail!(err),
        }

        Ok(())
    }

    #[instrument(skip(tx, meili_client))]
    pub(crate) async fn commit(self, tx: Transaction<'_, Database>, meili_client: &MeiliClient) -> Result<()> {
        if let Err(err) = tx.commit().await {
            if let Some((path, trash_path)) = &self.moved
                && let Err(err) = fs::rename(trash_path, path).await
            {
                error!(?err, %trash_path, %path, "failed to restore directory after failed delete");
            }

            bail!(err);
        }

        let task = DeletedRepoCleanupTask {
            trash_path: self.moved.map(|(_, trash_path)| trash_path),
            s3_keys: self.s3_keys,
        };

        if let Err(err) = GLOBAL_QUEUE
            .get()
            .ok_or_else(|| anyhow!("deleted repo cleanup task should only be scheduled after queue has been initialized"))?
            .insert_task(&task as &dyn AsyncRunnable)
            .await
        {
            error!(repo_ids = ?self.repo_ids, ?err, "failed to schedule deleted repo cleanup task");
        }

        if let Some(client) = meili_client {
            if !self.repo_ids.is_empty()
                && let Err(err) = client.index(REPOS_MEILI_INDEX).delete_documents(&self.repo_ids).await
            {
                error!(?err, "failed to delete repos from meilisearch");
            }

            if !self.issue_ids.is_empty()
                && let Err(err) = client.index(ISSUES_MEILI_INDEX).delete_documents(&self.issue_ids).await
            {
                error!(?err, "failed to delete issues of repos from meilisearch");
            }
        }

        Ok(())
    }
}
