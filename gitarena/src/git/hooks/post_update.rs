use crate::TASK_DB_POOL;
use crate::git::GIT_CLI_AVAILABLE;
use crate::git::hooks::detect_languages::detect_languages;
use crate::git::hooks::detect_license::detect_license;
use crate::meili::MEILI_CLIENT;
use crate::prelude::MapToFangError;
use crate::repository::Repository;
use anyhow::Result;
use async_trait::async_trait;
use fang::{AsyncQueueable, AsyncRunnable, FangError, typetag};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;
use tokio::runtime::Handle;
use tokio::task::spawn_blocking;
use tracing::{debug, instrument, warn};
use uuid::Uuid;

pub(crate) static POST_UPDATE_TASK_TYPE: &str = "post_update";

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub(crate) struct PostUpdateTask {
    pub(crate) repo_id: Uuid,
    pub(crate) head_updated: bool,
}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for PostUpdateTask {
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
            debug!(repo_id = %self.repo_id, "repo not found, skipping post update");
            return Ok(());
        };

        let path = repo.get_fs_path(&mut tx).await.fang()?;

        tx.commit().await.fang()?;

        if *GIT_CLI_AVAILABLE {
            let status = Command::new("git")
                .args(["-c", "gc.autoDetach=false", "gc", "--auto", "--quiet"])
                .current_dir(&path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;

            match status {
                Ok(status) if !status.success() => {
                    let exit_code = status.code().map_or_else(|| "unknown".to_string(), |code| code.to_string());
                    warn!(exit_code, "Git garbage collector exited with non-zero status");
                }
                Ok(_) => {}
                Err(err) => warn!(?err, "Failed to execute Git garbage collector"),
            }
        }

        if !self.head_updated {
            return Ok(());
        }

        let repo = spawn_blocking(move || Handle::current().block_on(detect_repo_metadata(repo, path)))
            .await
            .fang()?
            .fang()?;

        let mut tx = db_pool.begin().await.fang()?;

        sqlx::query("update repositories set license = $1, languages = $2 where id = $3")
            .bind(&repo.license)
            .bind(&repo.languages)
            .bind(repo.id)
            .execute(&mut *tx)
            .await
            .fang()?;

        tx.commit().await.fang()?;

        if let Some(meili_client) = MEILI_CLIENT.get() {
            repo.index_meili(meili_client).await;
        }

        Ok(())
    }

    fn task_type(&self) -> String {
        POST_UPDATE_TASK_TYPE.to_string()
    }

    fn uniq(&self) -> bool {
        true
    }

    fn max_retries(&self) -> i32 {
        3
    }
}

async fn detect_repo_metadata(mut repo: Repository, path: String) -> Result<Repository> {
    let gitoxide_repo = gix::open(path)?;
    let store = gitoxide_repo.objects.store().clone();

    if let Err(err) = detect_license(store.clone(), &gitoxide_repo, &mut repo).await {
        warn!(repo.id = %repo.id, ?err, "Failed to detect license in repo");
    }

    if let Err(err) = detect_languages(store, &gitoxide_repo, &mut repo).await {
        warn!(repo.id = %repo.id, ?err, "Failed to detect languages in repo");
    }

    Ok(repo)
}
