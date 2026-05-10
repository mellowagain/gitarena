use crate::git::utils::repo_files_at_ref;
use crate::repository::Repository;

use actix_web::web::Data;
use actix_web::{HttpResponse, Responder};
use anyhow::{Result, bail};
use bstr::ByteSlice;
use gitarena_common::database::{Database, Pool};
use gitarena_macros::route;
use gix::objs::Tree;
use gix::refs::file::find::existing::Error as GitoxideFindError;
use serde::Serialize;
use sqlx::Transaction;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
struct RepoMetaResponse {
    #[serde(flatten)]
    repo: Repository,
    /// Whether the repository has no content yet
    empty: bool,
    /// File name of the readme on the default branch
    #[serde(skip_serializing_if = "Option::is_none")]
    readme: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "Repository metadata", body = RepoMetaResponse),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}", method = "GET", err = "json")]
pub(crate) async fn meta(repo: Repository, db: Data<Pool>) -> Result<impl Responder> {
    let mut tx = db.begin().await?;

    let (empty, readme) = find_readme_file_name(&repo, &mut tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(RepoMetaResponse { repo, empty, readme }))
}

async fn find_readme_file_name(repo: &Repository, tx: &mut Transaction<'_, Database>) -> Result<(bool, Option<String>)> {
    let gitoxide_repo = repo.gitoxide(tx).await?;

    let mut buffer = Vec::<u8>::new();
    let store = gitoxide_repo.objects.store().clone();

    let reference = match gitoxide_repo.refs.find_loose(repo.default_branch.as_str()) {
        Ok(reference) => reference,
        Err(GitoxideFindError::Find(err)) => bail!(err),
        Err(GitoxideFindError::NotFound { .. }) => return Ok((true, None)),
    };

    let tree_ref = repo_files_at_ref(&reference, store, &gitoxide_repo, &mut buffer).await?;
    let tree = Tree::from(tree_ref);

    let readme = tree
        .entries
        .iter()
        .find(|entry| entry.filename.to_lowercase().starts_with(b"readme"))
        .and_then(|entry| entry.filename.to_str().ok())
        .map(ToOwned::to_owned);

    Ok((false, readme))
}
