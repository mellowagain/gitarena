use crate::database::Pool;
use crate::git::GIT_CLI_AVAILABLE;
use crate::repo_store::{DiskUsage, LocalRepo, RepoStore, WriteLease};
use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use fs_extra::dir::{self, CopyOptions};
use git2::build::RepoBuilder;
use git2::{FetchOptions, RemoteCallbacks, Repository as Git2Repository, RepositoryInitOptions};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use tokio::task::spawn_blocking;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct LocalStore {
    base_dir: PathBuf,
}

impl LocalStore {
    pub(crate) fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    fn repo_path(&self, repo_id: Uuid) -> PathBuf {
        self.base_dir.join(repo_id.to_string())
    }

    fn trash_dir(&self) -> PathBuf {
        self.base_dir.join(".deleted")
    }

    fn trash_path(&self, repo_id: Uuid) -> PathBuf {
        self.trash_dir().join(repo_id.to_string())
    }

    #[instrument(err, skip(self, db_pool))]
    pub(crate) async fn migrate_layout(&self, db_pool: &Pool) -> Result<()> {
        let mut tx = db_pool.begin().await?;

        let repos: Vec<(Uuid, String, String)> = sqlx::query_as(
            "select r.id, coalesce(u.username, o.name), r.name from repositories r \
            left join users u on u.id = r.owner_user \
            left join organizations o on o.id = r.owner_org",
        )
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;

        let mut namespaces = Vec::new();

        for (id, namespace, name) in repos {
            let legacy_path = self.base_dir.join(&namespace).join(&name);
            let new_path = self.repo_path(id);

            if fs::try_exists(&legacy_path).await? && !fs::try_exists(&new_path).await? {
                fs::rename(&legacy_path, &new_path).await?;
                info!(%id, from = %legacy_path.display(), to = %new_path.display(), "moved repository to id-based layout");
            }

            namespaces.push(namespace);
        }

        namespaces.sort_unstable();
        namespaces.dedup();

        for namespace in namespaces {
            // only succeeds if the directory is empty, which is exactly when the namespace has been fully migrated
            if let Err(err) = fs::remove_dir(self.base_dir.join(&namespace)).await
                && err.kind() != ErrorKind::NotFound
            {
                debug!(?err, namespace, "legacy namespace directory not removed");
            }
        }

        Ok(())
    }
}

#[async_trait]
impl RepoStore for LocalStore {
    #[instrument(err, skip(self))]
    async fn init(&self, repo_id: Uuid, default_branch: &str) -> Result<()> {
        let path = self.repo_path(repo_id);
        let default_branch = default_branch.to_owned();

        spawn_blocking(move || {
            let mut init_ops = RepositoryInitOptions::new();
            init_ops.initial_head(default_branch.as_str());
            init_ops.bare(true);

            Git2Repository::init_opts(path, &init_ops)?;
            Ok(())
        })
        .await?
    }

    #[instrument(err, skip(self, credentials))]
    async fn import(&self, repo_id: Uuid, url: &str, credentials: Option<(String, String)>) -> Result<()> {
        let path = self.repo_path(repo_id);
        let url = url.to_owned();

        spawn_blocking(move || {
            let mut callbacks = RemoteCallbacks::new();

            callbacks.credentials(move |_, _, _| {
                if let Some((username, password)) = &credentials {
                    git2::Cred::userpass_plaintext(username, password)
                } else {
                    git2::Cred::default()
                }
            });

            let mut fetch_opts = FetchOptions::new();
            fetch_opts.remote_callbacks(callbacks);

            RepoBuilder::new().bare(true).fetch_options(fetch_opts).clone(&url, &path)?;
            Ok(())
        })
        .await?
    }

    #[instrument(err, skip(self))]
    async fn fork(&self, source: Uuid, target: Uuid) -> Result<()> {
        let source = self.repo_path(source);
        let target = self.repo_path(target);

        spawn_blocking(move || {
            dir::create_all(&target, false)?;
            dir::copy(&source, &target, &CopyOptions::new().content_only(true)).context("failed to copy repository")?;
            Ok(())
        })
        .await?
    }

    async fn open_read(&self, repo_id: Uuid) -> Result<LocalRepo> {
        Ok(LocalRepo { path: self.repo_path(repo_id) })
    }

    async fn open_write(&self, repo_id: Uuid) -> Result<WriteLease> {
        Ok(WriteLease {
            local: self.open_read(repo_id).await?,
        })
    }

    async fn commit(&self, _lease: WriteLease) -> Result<()> {
        Ok(())
    }

    #[instrument(err, skip(self))]
    async fn trash(&self, repo_id: Uuid) -> Result<()> {
        fs::create_dir_all(self.trash_dir()).await?;

        match fs::rename(self.repo_path(repo_id), self.trash_path(repo_id)).await {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => bail!(err),
        }
    }

    #[instrument(err, skip(self))]
    async fn restore(&self, repo_id: Uuid) -> Result<()> {
        match fs::rename(self.trash_path(repo_id), self.repo_path(repo_id)).await {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => bail!(err),
        }
    }

    #[instrument(err, skip(self))]
    async fn purge(&self, repo_id: Uuid) -> Result<()> {
        match fs::remove_dir_all(self.trash_path(repo_id)).await {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => bail!(err),
        }
    }

    async fn size(&self, repo_id: Uuid) -> Result<u64> {
        Ok(dir::get_size(self.repo_path(repo_id))?)
    }

    async fn disk_usage(&self) -> Result<DiskUsage> {
        let stats = fs2::statvfs(&self.base_dir).context("failed to get statvfs for the repos directory")?;

        Ok(DiskUsage {
            total: stats.total_space(),
            available: stats.available_space(),
        })
    }

    #[instrument(err, skip(self))]
    async fn maintenance(&self, repo_id: Uuid) -> Result<()> {
        if !*GIT_CLI_AVAILABLE {
            return Ok(());
        }

        let status = Command::new("git")
            .args(["-c", "gc.autoDetach=false", "gc", "--auto", "--quiet"])
            .current_dir(self.repo_path(repo_id))
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

        Ok(())
    }
}
