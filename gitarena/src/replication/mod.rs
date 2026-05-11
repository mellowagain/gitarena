use crate::TASK_DB_POOL;
use crate::prelude::MapToFangError;
use crate::repository::Repository;
use async_trait::async_trait;
use fang::{AsyncQueueable, AsyncRunnable, Deserialize, FangError, Serialize, typetag};
use git2::build::RepoBuilder;
use git2::{FetchOptions, RemoteCallbacks};
use std::path::PathBuf;
use tokio::task::spawn_blocking;
use tracing::{debug, instrument};

#[derive(derive_more::Debug, Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub(crate) struct ImportTask {
    pub(crate) source: String,
    pub(crate) target: Repository,

    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for ImportTask {
    #[instrument(skip(_client))]
    async fn run(&self, _client: &dyn AsyncQueueable) -> Result<(), FangError> {
        let db_pool = TASK_DB_POOL.get().ok_or_else(|| FangError {
            description: "task db pool is empty".to_string(),
        })?;

        let mut tx = db_pool.begin().await.fang()?;
        let path_str = self.target.get_fs_path(&mut tx).await.fang()?;
        tx.commit().await.fang()?;

        let path = PathBuf::from(path_str);
        let source = self.source.clone();
        let username = self.username.clone();
        let password = self.password.clone();

        spawn_blocking(move || {
            debug!(%source, "starting git import");

            let mut callbacks = RemoteCallbacks::new();

            callbacks.credentials(move |_, _, _| {
                if let Some(username) = &username
                    && let Some(password) = &password
                {
                    git2::Cred::userpass_plaintext(username, password)
                } else {
                    git2::Cred::default()
                }
            });

            let mut fetch_opts = FetchOptions::new();
            fetch_opts.remote_callbacks(callbacks);

            RepoBuilder::new().bare(true).fetch_options(fetch_opts).clone(&source, &path).fang()?;

            debug!(%source, "git import succeeded");
            Ok::<_, FangError>(())
        })
        .await
        .fang()??;

        Ok(())
    }

    fn max_retries(&self) -> i32 {
        3
    }
}
