use crate::storage::STORAGE;
use async_trait::async_trait;
use fang::{AsyncQueueable, AsyncRunnable, FangError, typetag};
use futures::{StreamExt, TryStreamExt, stream};
use object_store::ObjectStore;
use object_store::path::Path;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use tokio::fs;
use tracing::instrument;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub(crate) struct DeletedRepoCleanupTask {
    pub(crate) trash_path: Option<String>,
    pub(crate) s3_keys: Vec<String>,
}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for DeletedRepoCleanupTask {
    #[instrument(skip(_client))]
    async fn run(&self, _client: &dyn AsyncQueueable) -> Result<(), FangError> {
        if let Some(trash_path) = &self.trash_path
            && let Err(err) = fs::remove_dir_all(trash_path).await
            && err.kind() != ErrorKind::NotFound
        {
            return Err(FangError {
                description: format!("failed to remove {trash_path}: {err}"),
            });
        }

        if self.s3_keys.is_empty() {
            return Ok(());
        }

        let Some(storage) = STORAGE.get() else {
            return Err(FangError {
                description: "storage OnceCell is empty".to_string(),
            });
        };

        let Some(store) = storage else {
            return Ok(());
        };

        let locations = stream::iter(self.s3_keys.clone()).map(|key| Ok(Path::from(key))).boxed();

        store.delete_stream(locations).try_collect::<Vec<_>>().await.map_err(|err| FangError {
            description: format!("failed to delete release assets from s3: {err}"),
        })?;

        Ok(())
    }

    fn max_retries(&self) -> i32 {
        3
    }
}
