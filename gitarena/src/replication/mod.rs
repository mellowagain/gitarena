use crate::prelude::MapToFangError;
use crate::repo_store;
use crate::repository::Repository;
use async_trait::async_trait;
use fang::{AsyncQueueable, AsyncRunnable, Deserialize, FangError, Serialize, typetag};
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
        let credentials = self.username.clone().zip(self.password.clone());

        debug!(source = %self.source, "starting git import");
        repo_store::get().fang()?.import(self.target.id, &self.source, credentials).await.fang()?;
        debug!(source = %self.source, "git import succeeded");

        Ok(())
    }

    fn max_retries(&self) -> i32 {
        3
    }
}
