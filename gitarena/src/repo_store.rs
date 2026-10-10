use crate::database::Pool;
use crate::repo_store::local::LocalStore;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use derive_more::Deref;
use git2::Repository as Git2Repository;
use gitarena_macros::from_config;
use gix::Repository as GitoxideRepository;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::OnceCell;
use uuid::Uuid;

pub(crate) mod local;

pub(crate) static REPO_STORE: OnceCell<Arc<dyn RepoStore>> = OnceCell::const_new();

/// If `commit` fails, the implementation MUST invalidate its local copy so a diverged repository is never served.
#[async_trait]
pub(crate) trait RepoStore: Send + Sync {
    async fn init(&self, repo_id: Uuid, default_branch: &str) -> Result<()>;
    async fn import(&self, repo_id: Uuid, url: &str, credentials: Option<(String, String)>) -> Result<()>;
    async fn fork(&self, source: Uuid, target: Uuid) -> Result<()>;

    async fn open_read(&self, repo_id: Uuid) -> Result<LocalRepo>;
    async fn open_write(&self, repo_id: Uuid) -> Result<WriteLease>;
    async fn commit(&self, lease: WriteLease) -> Result<()>;

    async fn trash(&self, repo_id: Uuid) -> Result<()>;
    async fn restore(&self, repo_id: Uuid) -> Result<()>;
    async fn purge(&self, repo_id: Uuid) -> Result<()>;

    async fn size(&self, repo_id: Uuid) -> Result<u64>;
    async fn disk_usage(&self) -> Result<DiskUsage>;
    async fn maintenance(&self, repo_id: Uuid) -> Result<()>;
}

#[derive(Debug)]
pub(crate) struct LocalRepo {
    path: PathBuf,
}

impl LocalRepo {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn libgit2(&self) -> Result<Git2Repository> {
        Ok(Git2Repository::open(&self.path)?)
    }

    pub(crate) fn gitoxide(&self) -> Result<GitoxideRepository> {
        Ok(gix::open(&self.path)?)
    }
}

#[derive(Debug, Deref)]
pub(crate) struct WriteLease {
    local: LocalRepo,
}

impl WriteLease {
    pub(crate) async fn commit(self) -> Result<()> {
        get()?.commit(self).await
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct DiskUsage {
    pub(crate) total: u64,
    pub(crate) available: u64,
}

pub(crate) async fn init(db_pool: &Pool) -> Result<Arc<dyn RepoStore>> {
    let base_dir = from_config!("repositories.base_dir" => String);

    let store = LocalStore::new(PathBuf::from(base_dir));
    store.migrate_layout(db_pool).await?;

    Ok(Arc::new(store))
}

pub(crate) fn get() -> Result<&'static dyn RepoStore> {
    REPO_STORE
        .get()
        .map(AsRef::as_ref)
        .ok_or_else(|| anyhow!("repo store should be initialized on startup"))
}
