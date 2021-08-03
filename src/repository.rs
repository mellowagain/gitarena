use crate::user::User;
use crate::config::CONFIG;

use std::borrow::Borrow;

use anyhow::Result;
use git2::{Repository as Git2Repository, RepositoryInitOptions};
use sqlx::{FromRow, Postgres, Transaction};

#[derive(FromRow)]
pub(crate) struct Repository {
    pub(crate) id: i32,
    pub(crate) owner: i32,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) private: bool
}

impl Repository {
    pub(crate) async fn create_fs(&self, owner_username: &str) -> Result<()> {
        let mut init_ops = RepositoryInitOptions::new();
        init_ops.initial_head("main");
        init_ops.no_reinit(true);

        Git2Repository::init_opts(self.get_fs_path(owner_username).await, &init_ops)?;

        Ok(())
    }

    pub(crate) async fn libgit2(&self, owner_username: &str) -> Result<Git2Repository> {
        Ok(Git2Repository::open(self.get_fs_path(owner_username).await)?)
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
