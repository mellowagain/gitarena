use crate::privileges::repo_visibility::RepoVisibility;

use anyhow::Result;
use derive_more::Display;
use fs_extra::dir;
use git2::{Repository as Git2Repository, RepositoryInitOptions};
use git_repository::Repository as GitoxideRepository;
use serde::Serialize;
use sqlx::{Executor, FromRow, Postgres};

#[derive(FromRow, Display, Debug, Serialize)]
#[display(fmt = "{}", name)]
pub(crate) struct Repository {
    pub(crate) id: i32,

    pub(crate) owner: i32,
    pub(crate) name: String,
    pub(crate) description: String,

    pub(crate) visibility: RepoVisibility,
    pub(crate) default_branch: String,

    pub(crate) license: Option<String>,

    pub(crate) forked_from: Option<i32>,
    pub(crate) mirrored_from: Option<String>,

    pub(crate) archived: bool,
    pub(crate) disabled: bool
}

impl Repository {
    pub(crate) async fn open<'e, E, I, S>(user_id: I, repo_name: S, executor: E) -> Option<Repository>
        where E: Executor<'e, Database = Postgres>,
              I: Into<i32>,
              S: AsRef<str>
    {
        let user_id = user_id.into();
        let repo_name = repo_name.as_ref();

        let repo: Option<Repository> = sqlx::query_as::<_, Repository>("select * from repositories where owner = $1 and lower(name) = lower($2)")
            .bind(&user_id)
            .bind(repo_name)
            .fetch_optional(executor)
            .await
            .ok()
            .flatten();

        repo
    }

    pub(crate) async fn create_fs<'e, E: Executor<'e, Database = Postgres>>(&self, executor: E) -> Result<()> {
        let mut init_ops = RepositoryInitOptions::new();
        init_ops.initial_head(self.default_branch.as_str());
        init_ops.no_reinit(true);
        init_ops.bare(true);

        Git2Repository::init_opts(self.get_fs_path(executor).await?, &init_ops)?;

        Ok(())
    }

    pub(crate) async fn libgit2<'e, E: Executor<'e, Database = Postgres>>(&self, executor: E) -> Result<Git2Repository> {
        Ok(Git2Repository::open(self.get_fs_path(executor).await?)?)
    }

    pub(crate) async fn gitoxide<'e, E: Executor<'e, Database = Postgres>>(&self, executor: E) -> Result<GitoxideRepository> {
        Ok(GitoxideRepository::discover(self.get_fs_path(executor).await?)?)
    }

    pub(crate) async fn get_fs_path<'e, E: Executor<'e, Database = Postgres>>(&self, executor: E) -> Result<String> {
        // Instead of using `config::get_optional_setting`, we run our own query to get both username and repo base dir in one query
        // https://stackoverflow.com/a/16364390
        let (base_dir, username): (String, String) = sqlx::query_as(
            "select * from \
            (select value from settings where key = 'repositories.base_dir') A \
            cross join \
            (select username from users where id = $1) B"
        )
            .bind(&self.owner)
            .fetch_one(executor)
            .await?;

        Ok(format!("{}/{}/{}", base_dir, username, &self.name))
    }

    pub(crate) async fn repo_size<'e, E: Executor<'e, Database = Postgres>>(&self, executor: E) -> Result<u64> {
        Ok(dir::get_size(self.get_fs_path(executor).await?)?)
    }
}
