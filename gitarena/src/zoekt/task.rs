use crate::prelude::MapToFangError;
use crate::repository::Repository;
use crate::utils::time_function;
use crate::{TASK_DB_POOL, config};
use anyhow::Context;
use async_trait::async_trait;
use fang::{AsyncQueueable, AsyncRunnable, FangError, typetag};
use gix::path::env;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{debug, instrument};

pub(crate) static ZOEKT_TASK_TYPE: &str = "zoekt";

#[derive(derive_more::Debug, Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub(crate) struct ZoektIndexRepo {
    pub(crate) repo: Repository,
}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for ZoektIndexRepo {
    #[instrument(skip(_client))]
    async fn run(&self, _client: &dyn AsyncQueueable) -> Result<(), FangError> {
        let db_pool = TASK_DB_POOL.get().ok_or_else(|| FangError {
            description: "task db pool OnceCell is empty".to_string(),
        })?;

        let mut tx = db_pool.begin().await.fang()?;

        let zoekt_index_dir: String = config::get_setting("zoekt.index_dir", &mut tx).await.fang()?;
        let mut zoekt_index_binary_path: String = config::get_setting("zoekt.index_binary_path", &mut tx).await.fang()?;

        let path = self.repo.get_fs_path(&mut tx).await.fang()?;

        tx.commit().await.fang()?;

        let home = env::var("HOME").unwrap_or_default();
        zoekt_index_binary_path = zoekt_index_binary_path.replacen('~', home.to_str().unwrap_or_default(), 1);

        let mut command = Command::new(zoekt_index_binary_path)
            .args(["-branches", &self.repo.default_branch])
            .args(["-index", &zoekt_index_dir])
            .args(["-submodules=false"])
            .arg(path)
            .spawn()
            .context("failed to spawn zoekt-git-index child process")
            .fang()?;

        let (time_ms, result) = time_function(|| async { command.wait().await }).await;
        let exit_status = result.context("zoekt-git-index child process failed to finish").fang()?;

        if !exit_status.success() {
            let code = exit_status.code().map_or_else(|| "terminated by signal".to_string(), |code| code.to_string());

            return Err(FangError {
                description: format!("zoekt-git-index exited with non-zero exit code: {code}"),
            });
        }

        debug!(%time_ms, "successfully indexed repo in zoekt");
        Ok(())
    }

    fn task_type(&self) -> String {
        ZOEKT_TASK_TYPE.to_string()
    }
}
