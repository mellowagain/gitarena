use crate::err;
use crate::git::utils::{read_blob_content, repo_files_at_ref};
use crate::repository::{Branch, Repository};

use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use bstr::ByteSlice;
use gitarena_macros::route;
use gix::objs::Tree;
use serde::Serialize;
use utoipa::ToSchema;

/// Readme file name and rendered content
#[derive(Serialize, ToSchema)]
pub(crate) struct ReadmeResponse {
    /// File name of the readme
    file_name: String,
    /// Raw text content of the readme file
    content: String,
}

#[utoipa::path(
    get,
    path = "/api/repo/{namespace}/{repository}/tree/{tree}/readme",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("tree" = String, Path, description = "Branch or tag name"),
    ),
    responses(
        (status = 200, description = "Readme file name and content", body = ReadmeResponse),
        (status = 404, description = "Repository, branch, or readme not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repo/{namespace}/{repository}/tree/{tree:.*}/readme", method = "GET", err = "json")]
pub(crate) async fn readme(_repo: Repository, branch: Branch) -> Result<impl Responder> {
    let gitoxide_repo = branch.gitoxide_repo;

    let mut buffer = Vec::<u8>::new();
    let store = gitoxide_repo.objects.store().clone();

    let tree_ref = repo_files_at_ref(&branch.reference, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let tree = Tree::from(tree_ref);

    let entry = tree
        .entries
        .iter()
        .find(|e| e.filename.to_lowercase().starts_with(b"readme"))
        .ok_or_else(|| err!(NOT_FOUND, "No readme file found"))?;

    let file_name = entry.filename.to_str().unwrap_or("Invalid file name").to_owned();
    let content = read_blob_content(entry.oid.as_ref(), store).await?;

    Ok(HttpResponse::Ok().json(ReadmeResponse { file_name, content }))
}
