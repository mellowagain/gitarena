use crate::error::{ErrorDisplayType, GitArenaError};
use crate::organization::Organization;
use crate::privileges::privilege;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::user::{User, WebUser};
use crate::{die, err};

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::database::Database;
use crate::database::Pool;
use actix_web::dev::Payload;
use actix_web::web::Data;
use actix_web::{FromRequest, HttpMessage, HttpRequest};
use anyhow::{Result, anyhow};
use derive_more::{Deref, Display};
use fs_extra::dir;
use git2::{Repository as Git2Repository, RepositoryInitOptions};
use gix::Repository as GitoxideRepository;
use gix::refs::file::find::existing::Error as GitoxideFindError;
use gix::refs::file::loose::Reference;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{FromRow, Transaction};
use tracing::{Level, instrument};
use tracing_unwrap::OptionExt;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(FromRow, Display, Clone, derive_more::Debug, Serialize, Deserialize, ToSchema)]
#[display("{name}")]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct Repository {
    /// ID
    pub(crate) id: Uuid,
    /// UUID of the user who owns this repository
    pub(crate) owner_user: Option<Uuid>,
    /// UUID of the organization that owns this repository
    pub(crate) owner_org: Option<Uuid>,
    /// Name of the repository
    pub(crate) name: String,
    /// Description, set by the user
    pub(crate) description: String,
    /// Visibility
    pub(crate) visibility: RepoVisibility,
    /// Default branch
    pub(crate) default_branch: String,
    /// Auto-detected license name
    pub(crate) license: Option<String>,
    /// Auto-detected programming language stats of the main branch in the format of (Language, Byte count).
    /// Caveat: SVG files are the amount of files instead of the total byte count to avoid skewing repository stats
    #[debug("{}", languages.len())]
    #[schema(value_type = HashMap<String, u64>)]
    pub(crate) languages: Json<HashMap<String, u64>>,
    /// ID of the repo from which this one was forked from
    pub(crate) forked_from: Option<Uuid>,
    /// URL of the repo from which this one is mirrored from
    #[debug("{}", mirrored_from.is_some())]
    pub(crate) mirrored_from: Option<String>,
    /// Archived flag
    pub(crate) archived: bool,
    /// Disabled flag
    pub(crate) disabled: bool,
}

impl Repository {
    pub(crate) async fn open(owner_id: Uuid, repo_name: impl AsRef<str>, tx: &mut Transaction<'_, Database>) -> Option<Repository> {
        let repo_name = repo_name.as_ref();

        let repo: Option<Repository> =
            sqlx::query_as::<_, Repository>("select * from repositories where (owner_user = $1 or owner_org = $1) and lower(name) = lower($2) limit 1")
                .bind(owner_id)
                .bind(repo_name)
                .fetch_optional(&mut **tx)
                .await
                .ok()
                .flatten();

        repo
    }

    #[instrument(err, skip(tx))]
    pub(crate) async fn create_fs(&self, tx: &mut Transaction<'_, Database>) -> Result<()> {
        let mut init_ops = RepositoryInitOptions::new();
        init_ops.initial_head(self.default_branch.as_str());
        init_ops.bare(true);

        Git2Repository::init_opts(self.get_fs_path(tx).await?, &init_ops)?;

        Ok(())
    }

    #[instrument(err, skip(tx))]
    pub(crate) async fn libgit2(&self, tx: &mut Transaction<'_, Database>) -> Result<Git2Repository> {
        Ok(Git2Repository::open(self.get_fs_path(tx).await?)?)
    }

    #[instrument(err, skip(tx))]
    pub(crate) async fn gitoxide(&self, tx: &mut Transaction<'_, Database>) -> Result<GitoxideRepository> {
        Ok(gix::discover(self.get_fs_path(tx).await?)?)
    }

    #[instrument(ret(level = Level::DEBUG), err, skip(tx))]
    pub(crate) async fn get_fs_path(&self, tx: &mut Transaction<'_, Database>) -> Result<String> {
        let owner_id = self
            .owner_user
            .or(self.owner_org)
            .ok_or_else(|| anyhow!("Repository has neither owner_user nor owner_org set"))?;

        // Instead of using `config::get_optional_setting`, we run our own query to get both namespace and repo base dir in one query
        // https://stackoverflow.com/a/16364390
        // The owner UUID may refer to either a user or an organization; try both tables.
        let user_result: Option<(String, String)> = sqlx::query_as(
            "select * from \
            (select value from settings where key = 'repositories.base_dir' limit 1) A \
            cross join \
            (select username as namespace from users where id = $1 limit 1) B",
        )
        .bind(owner_id)
        .fetch_optional(&mut **tx)
        .await?;

        let (base_dir, namespace) = if let Some(row) = user_result {
            row
        } else {
            sqlx::query_as(
                "select * from \
                (select value from settings where key = 'repositories.base_dir' limit 1) A \
                cross join \
                (select name as namespace from organizations where id = $1 limit 1) B",
            )
            .bind(owner_id)
            .fetch_one(&mut **tx)
            .await?
        };

        Ok(format!("{base_dir}/{namespace}/{}", &self.name))
    }

    #[instrument(ret(level = Level::DEBUG), err, skip(tx))]
    pub(crate) async fn repo_size(&self, tx: &mut Transaction<'_, Database>) -> Result<u64> {
        Ok(dir::get_size(self.get_fs_path(tx).await?)?)
    }
}

impl FromRequest for Repository {
    type Error = GitArenaError;
    type Future = Pin<Box<dyn Future<Output = Result<Repository, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let match_info = req.match_info();

        // If this method gets called from a handler that does not have namespace or repository in the match info
        // it is safe to assume the programmer made a mistake, thus .expect_or_log is OK
        let namespace = match_info
            .get("namespace")
            .or_else(|| match_info.get("username"))
            .expect_or_log("from_request called on Repository despite not having namespace/username argument")
            .to_owned();

        let repository = match_info
            .get("repository")
            .expect_or_log("from_request called on Repository despite not having repository argument")
            .to_owned();

        // Allows one to receive the repo owner name without having to manually search the database
        // This .clone is most likely unnecessary as the previous value is only used as-ref below
        req.extensions_mut().insert(RepoOwner(namespace.clone()));

        let web_user_future = WebUser::from_request(req, payload);

        match req.app_data::<Data<Pool>>() {
            Some(db_pool) => {
                let db_pool = db_pool.clone();

                Box::pin(async move {
                    let web_user = web_user_future.await?;

                    extract_repo_from_request(&db_pool, web_user.as_ref(), namespace.as_str(), repository.as_str())
                        .await
                        .map_err(|err| GitArenaError {
                            source: Arc::new(err),
                            display_type: ErrorDisplayType::Json, // TODO: Check whenever route is err = "json|git" etc...
                        })
                })
            }
            None => Box::pin(async {
                Err(GitArenaError {
                    source: Arc::new(anyhow!("No PgPool in application data")),
                    display_type: ErrorDisplayType::Json, // TODO: Check whenever route is err = "json|git" etc...
                })
            }),
        }
    }
}

#[instrument(err, skip(db_pool))]
pub(crate) async fn extract_repo_from_request(db_pool: &Pool, actor: Option<&User>, namespace: &str, repository: &str) -> Result<Repository> {
    let mut tx = db_pool.begin().await?;

    // The namespace may refer to either a user or an organization.
    let owner_id: Uuid = if let Some(user) = User::find_using_name(namespace, &mut tx).await {
        user.id
    } else if let Some(org) = Organization::find_by_name(namespace, &mut tx).await {
        org.id
    } else {
        die!(NOT_FOUND, "Repository not found");
    };

    let repo = Repository::open(owner_id, repository, &mut tx)
        .await
        .ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;

    if !privilege::check_access(&repo, actor, &mut tx).await? {
        die!(NOT_FOUND, "Repository not found");
    }

    tx.commit().await?;

    Ok(repo)
}

/// Will only be part of [Extensions](actix_web::Extensions) if [Repository] is in the handler arguments
#[derive(Display, Debug, Deref)]
#[display("{_0}")]
pub(crate) struct RepoOwner(pub(crate) String);

#[derive(Display, Debug)]
#[display("{tree}")]
pub(crate) struct Branch {
    pub(crate) gitoxide_repo: GitoxideRepository,
    pub(crate) tree: String,
    pub(crate) reference: Reference,
}

impl FromRequest for Branch {
    type Error = GitArenaError;
    type Future = Pin<Box<dyn Future<Output = Result<Branch, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let match_info = req.match_info();

        // If this method gets called from a handler that does not have tree in the match info
        // it is safe to assume the programmer made a mistake, thus .expect_or_log is OK
        let tree = match_info
            .get("tree")
            .expect_or_log("from_request called on Branch despite not having tree argument")
            .to_owned();

        let repo_future = Repository::from_request(req, payload);

        match req.app_data::<Data<Pool>>() {
            Some(db_pool) => {
                let db_pool = db_pool.clone();

                Box::pin(async move {
                    // This call exists early if access rights are insufficient, so we don't need to worry about them down the road
                    let repo = repo_future.await?;

                    extract_branch_from_request(db_pool, repo, tree).await.map_err(|err| GitArenaError {
                        source: Arc::new(err),
                        display_type: ErrorDisplayType::Json, // TODO: Check whenever route is err = "json|git" etc...
                    })
                })
            }
            None => Box::pin(async {
                Err(GitArenaError {
                    source: Arc::new(anyhow!("No PgPool in application data")),
                    display_type: ErrorDisplayType::Json, // TODO: Check whenever route is err = "json|git" etc...
                })
            }),
        }
    }
}

#[instrument(err, skip(db_pool))]
async fn extract_branch_from_request(db_pool: Data<Pool>, repo: Repository, tree: String) -> Result<Branch> {
    let mut transaction = db_pool.begin().await?;

    let gitoxide_repo = repo.gitoxide(&mut transaction).await?;

    let reference = match gitoxide_repo.refs.find_loose(tree.as_str()) {
        Ok(loose_ref) => Ok(loose_ref),
        Err(GitoxideFindError::Find(err)) => Err(err),
        Err(GitoxideFindError::NotFound { name: _ }) => die!(NOT_FOUND, "Tree not found"),
    }?;

    transaction.commit().await?;

    Ok(Branch {
        gitoxide_repo,
        tree,
        reference,
    })
}
