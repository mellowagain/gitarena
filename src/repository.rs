use crate::user::User;
use crate::config::CONFIG;

use std::borrow::Borrow;
use std::path::Path;

use anyhow::Result;
use git2::Repository as LibGit2Repository;
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
        let repo_base_dir: &str = CONFIG.repositories.base_dir.borrow();
        let path_str = format!("{}/{}/{}", repo_base_dir, owner_username, &self.name);
        let path = Path::new(path_str.as_str());

        LibGit2Repository::init(path)?;

        Ok(())
    }

    pub(crate) async fn get_owner(&self, transaction: &mut Transaction<'_, Postgres>) -> Result<User> {
        Ok(sqlx::query_as::<_, User>("select * from users where id = $1 limit 1")
            .bind(self.owner)
            .fetch_one(transaction)
            .await?)
    }
}
