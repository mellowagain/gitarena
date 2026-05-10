use crate::err;
use crate::prelude::LibGit2SignatureExtensions;
use crate::repository::{Branch, Repository};
use std::path::Path;
use uuid::Uuid;

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use bstr::ByteSlice;
use git2::BlameOptions;
use gitarena_common::database::Pool;
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize)]
struct BlameRequest {
    namespace: String,
    repository: String,
    tree: String,
    file_path: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlameHunk {
    commit_id: String,
    author_name: String,
    /// GitArena user ID of the author, if matched
    author_uid: Option<Uuid>,
    author_email: String,
    /// Unix timestamp of the commit
    timestamp: i64,
    summary: String,
    /// 1-indexed starting line number in the final file
    start_line: u32,
    /// Number of lines belonging to this hunk
    num_lines: u32,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlameResponse {
    hunks: Vec<BlameHunk>,
}

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/branch/{tree}/blame/{file_path}",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("tree" = String, Path, description = "Branch name or full ref"),
        ("file_path" = String, Path, description = "File path relative to the repository root"),
    ),
    responses(
        (status = 200, description = "Blame hunks for the file", body = BlameResponse),
        (status = 404, description = "Repository, branch, or file not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/branch/{tree}/blame/{file_path:.*}", method = "GET", err = "json")]
pub(crate) async fn blame(repo: Repository, branch: Branch, uri: web::Path<BlameRequest>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let libgit2_repo = repo.libgit2(&mut transaction).await?;
    let full_tree_name = branch.reference.name.as_bstr().to_str()?;

    let reference = libgit2_repo.find_reference(full_tree_name).map_err(|_| err!(NOT_FOUND, "Branch not found"))?;

    let commit = reference.peel_to_commit().map_err(|_| err!(NOT_FOUND, "Branch tip is not a commit"))?;

    let mut opts = BlameOptions::new();
    opts.newest_commit(commit.id());

    let blame_result = libgit2_repo
        .blame_file(Path::new(uri.file_path.as_str()), Some(&mut opts))
        .map_err(|_| err!(NOT_FOUND, "File not found or could not be blamed"))?;

    let mut hunks = Vec::new();

    for hunk in blame_result.iter() {
        let oid = hunk.final_commit_id();
        let commit = libgit2_repo.find_commit(oid).map_err(|_| err!(NOT_FOUND, "Blame commit not found"))?;

        let (author_name, author_uid, author_email) = commit.author().try_disassemble(&mut transaction).await;
        let timestamp = commit.time().seconds();
        let summary = commit.message().unwrap_or_default().lines().next().unwrap_or_default().to_owned();

        hunks.push(BlameHunk {
            commit_id: format!("{oid}"),
            author_name,
            author_uid,
            author_email,
            timestamp,
            summary,
            start_line: u32::try_from(hunk.final_start_line()).map_err(|_| err!(BAD_REQUEST, "start line in hunk is larger than u32"))?,
            num_lines: u32::try_from(hunk.lines_in_hunk()).map_err(|_| err!(BAD_REQUEST, "line amount in hunk is larger than u32"))?,
        });
    }

    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(BlameResponse { hunks }))
}
