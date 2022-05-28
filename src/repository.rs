use crate::error::{ErrorDisplayType, GitArenaError};
use crate::privileges::privilege;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::user::{User, WebUser};
use crate::{die, err};

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use actix_web::dev::Payload;
use actix_web::web::Data;
use actix_web::{FromRequest, HttpRequest};
use anyhow::{anyhow, Result};
use derive_more::Display;
use fs_extra::dir;
use git2::{Repository as Git2Repository, RepositoryInitOptions};
use git_repository::refs::file::find::existing::Error as GitoxideFindError;
use git_repository::refs::file::loose::Reference;
use git_repository::Repository as GitoxideRepository;
use serde::Serialize;
use sqlx::{Executor, FromRow, PgPool, Postgres};
use tracing_unwrap::OptionExt;

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

        let repo: Option<Repository> = sqlx::query_as::<_, Repository>("select * from repositories where owner = $1 and lower(name) = lower($2) limit 1")
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
            (select value from settings where key = 'repositories.base_dir' limit 1) A \
            cross join \
            (select username from users where id = $1 limit 1) B"
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

impl FromRequest for Repository {
    type Error = GitArenaError;
    type Future = Pin<Box<dyn Future<Output = Result<Repository, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let match_info = req.match_info();

        // If this method gets called from a handler that does not have username or repository in the match info
        // it is safe to assume the programmer made a mistake, thus .expect_or_log is OK
        let username = match_info.get("username").expect_or_log("from_request called on Repository despite not having username argument").to_owned();
        let repository = match_info.get("repository").expect_or_log("from_request called on Repository despite not having repository argument").to_owned();
        //let tree = match_info.get("tree");

        let web_user_future = WebUser::from_request(req, payload);

        match req.app_data::<Data<PgPool>>() {
            Some(db_pool) => {
                // Data<PgPool> is just a wrapper around `Arc<P>` so .clone() is cheap
                let db_pool = db_pool.clone();

                Box::pin(async move {
                    let web_user = web_user_future.await?;

                    extract_repo_from_request(db_pool, web_user, username.as_str(), repository.as_str()).await.map_err(|err| GitArenaError {
                        source: Arc::new(err),
                        display_type: ErrorDisplayType::Html // TODO: Check whenever route is err = "html|json|git" etc...
                    })
                })
            }
            None => Box::pin(async {
                Err(GitArenaError {
                    source: Arc::new(anyhow!("No PgPool in application data")),
                    display_type: ErrorDisplayType::Html // TODO: Check whenever route is err = "html|json|git" etc...
                })
            })
        }
    }
}

async fn extract_repo_from_request(db_pool: Data<PgPool>, web_user: WebUser, username: &str, repository: &str) -> Result<Repository> {
    let mut transaction = db_pool.begin().await?;

    let user = User::find_using_name(username, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;
    let repo = Repository::open(user, repository, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;

    if !privilege::check_access(&repo, web_user.as_ref(), &mut transaction).await? {
        die!(NOT_FOUND, "Not found");
    }

    transaction.commit().await?;

    Ok(repo)
}

#[derive(Display, Debug)]
#[display(fmt = "{}", tree)]
pub(crate) struct Branch {
    gitoxide_repo: GitoxideRepository,
    tree: String,
    reference: Reference
}

impl FromRequest for Branch {
    type Error = GitArenaError;
    type Future = Pin<Box<dyn Future<Output = Result<Branch, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let match_info = req.match_info();

        // If this method gets called from a handler that does not have tree in the match info
        // it is safe to assume the programmer made a mistake, thus .expect_or_log is OK
        let tree = match_info.get("tree").expect_or_log("from_request called on Branch despite not having tree argument").to_owned();

        let repo_future = Repository::from_request(req, payload);

        match req.app_data::<Data<PgPool>>() {
            Some(db_pool) => {
                // Data<PgPool> is just a wrapper around `Arc<P>` so .clone() is cheap
                let db_pool = db_pool.clone();

                Box::pin(async move {
                    // This call exists early if access rights are insufficient, so we don't need to worry about them down the road
                    let repo = repo_future.await?;

                    extract_branch_from_request(db_pool, repo, tree).await.map_err(|err| GitArenaError {
                        source: Arc::new(err),
                        display_type: ErrorDisplayType::Html // TODO: Check whenever route is err = "html|json|git" etc...
                    })
                })
            }
            None => Box::pin(async {
                Err(GitArenaError {
                    source: Arc::new(anyhow!("No PgPool in application data")),
                    display_type: ErrorDisplayType::Html // TODO: Check whenever route is err = "html|json|git" etc...
                })
            })
        }
    }
}

async fn extract_branch_from_request(db_pool: Data<PgPool>, repo: Repository, tree: String) -> Result<Branch> {
    let mut transaction = db_pool.begin().await?;

    let gitoxide_repo = repo.gitoxide(&mut transaction).await?;

    let reference = match gitoxide_repo.refs.find_loose(tree.as_str()) {
        Ok(loose_ref) => Ok(loose_ref),
        Err(GitoxideFindError::Find(err)) => Err(err),
        Err(GitoxideFindError::NotFound(_)) => die!(NOT_FOUND, "Tree not found")
    }?;

    transaction.commit().await?;

    Ok(Branch {
        gitoxide_repo,
        tree,
        reference
    })
}
