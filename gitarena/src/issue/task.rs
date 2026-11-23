use crate::TASK_DB_POOL;
use crate::prelude::MapToFangError;
use crate::repository::Repository;
use async_trait::async_trait;
use fang::{AsyncQueueable, AsyncRunnable, FangError, typetag};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;
use tracing::{debug, instrument};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub(crate) struct WriteEmptyBlobTask {
    pub(crate) repo_id: Uuid,
}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for WriteEmptyBlobTask {
    #[instrument(skip(_client))]
    async fn run(&self, _client: &dyn AsyncQueueable) -> Result<(), FangError> {
        let db_pool = TASK_DB_POOL.get().ok_or_else(|| FangError {
            description: "task db pool OnceCell is empty".to_string(),
        })?;

        let mut tx = db_pool.begin().await.fang()?;

        let repo: Option<Repository> = sqlx::query_as("select * from repositories where id = $1 limit 1")
            .bind(self.repo_id)
            .fetch_optional(&mut *tx)
            .await
            .fang()?;

        let Some(repo) = repo else {
            debug!(repo_id = %self.repo_id, "repo not found, skipping empty blob write");
            return Ok(());
        };

        let git_repo = repo.gitoxide(&mut tx).await.fang()?;

        tx.commit().await.fang()?;

        spawn_blocking(move || git_repo.write_blob([]).map(|_| ())).await.fang()?.fang()?;

        Ok(())
    }

    fn max_retries(&self) -> i32 {
        3
    }
}
