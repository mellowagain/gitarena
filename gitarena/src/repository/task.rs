use crate::prelude::MapToFangError;
use crate::repo_store;
use crate::storage::STORAGE;
use async_trait::async_trait;
use fang::{AsyncQueueable, AsyncRunnable, FangError, typetag};
use futures::{StreamExt, TryStreamExt, stream};
use object_store::ObjectStore;
use object_store::path::Path;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub(crate) struct DeletedRepoCleanupTask {
    #[serde(default)]
    pub(crate) repo_ids: Vec<Uuid>,
    pub(crate) s3_keys: Vec<String>,
}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for DeletedRepoCleanupTask {
    #[instrument(skip(_client))]
    async fn run(&self, _client: &dyn AsyncQueueable) -> Result<(), FangError> {
        let store = repo_store::get().fang()?;

        for repo_id in &self.repo_ids {
            store.purge(*repo_id).await.fang()?;
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
