use crate::user::User;
use crate::config::CONFIG;

use std::borrow::Borrow;

use anyhow::Result;
use git2::{Repository as Git2Repository, RepositoryInitOptions};
use git_repository::Repository as GitoxideRepository;
use serde::Serialize;
use sqlx::{FromRow, Postgres, Transaction};

#[derive(FromRow, Serialize)]
pub(crate) struct Repository {
    pub(crate) id: i32,
    pub(crate) owner: i32,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) private: bool,
    pub(crate) default_branch: String,

    pub(crate) size: i64,
    pub(crate) license: Option<String>,
}

impl Repository {
    pub(crate) async fn create_fs(&self, owner_username: &str) -> Result<()> {
        let mut init_ops = RepositoryInitOptions::new();
        init_ops.initial_head(self.default_branch.as_str());
        init_ops.no_reinit(true);
        init_ops.bare(true);

        Git2Repository::init_opts(self.get_fs_path(owner_username).await, &init_ops)?;

        Ok(())
    }

    pub(crate) async fn libgit2(&self, owner_username: &str) -> Result<Git2Repository> {
        Ok(Git2Repository::open(self.get_fs_path(owner_username).await)?)
    }

    pub(crate) async fn gitoxide(&self, owner_username: &str) -> Result<GitoxideRepository> {
        Ok(GitoxideRepository::discover(self.get_fs_path(owner_username).await)?)
    }

    pub(crate) async fn get_owner(&self, transaction: &mut Transaction<'_, Postgres>) -> Result<User> {
        Ok(sqlx::query_as::<_, User>("select * from users where id = $1 limit 1")
            .bind(self.owner)
            .fetch_one(transaction)
            .await?)
    }

    pub(crate) async fn get_fs_path(&self, owner_username: &str) -> String {
        let repo_base_dir: &str = CONFIG.repositories.base_dir.borrow();

        format!("{}/{}/{}", repo_base_dir, owner_username, &self.name)
    }
}

impl Default for Repository {
    fn default() -> Repository {
        Repository {
            id: 0,
            owner: 0,
            name: "".to_owned(),
            description: "".to_owned(),
            private: false,
            default_branch: "main".to_owned(),
            size: 0,
            license: None
        }
    }
}
