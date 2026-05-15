use crate::git::history::{paged_commits, paged_commits_for_path};
use crate::prelude::LibGit2SignatureExtensions;
use crate::repository::{Branch, Repository};
use crate::routes::repository::api::branch_files::{CommitInfo, FileCommitInfo};

use crate::database::Pool;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use bstr::ByteSlice;
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

const MAX_LIMIT: usize = 100;

fn default_limit() -> usize {
    20
}

#[derive(Deserialize, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
enum SortOrder {
    #[default]
    Desc,
    Asc,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct CommitsQuery {
    #[param(default = 20, minimum = 1, maximum = 100)]
    #[serde(default = "default_limit")]
    limit: usize,
    #[param(minimum = 0)]
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    sort: SortOrder,
    /// Path to filter for
    #[serde(default)]
    path: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct BranchCommitsResponse {
    commits: Vec<FileCommitInfo>,
}

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/branch/{tree}/commits",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("tree" = String, Path, description = "Branch name (short form like 'main', or full ref like 'refs/heads/main')"),
        CommitsQuery,
    ),
    responses(
        (status = 200, description = "Paginated list of commits on the branch", body = BranchCommitsResponse),
        (status = 404, description = "Repository or branch not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/branch/{tree}/commits", method = "GET", err = "json")]
pub(crate) async fn branch_commits(repo: Repository, branch: Branch, query: web::Query<CommitsQuery>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let limit = query.limit.min(MAX_LIMIT);
    let reverse = matches!(query.sort, SortOrder::Asc);

    let libgit2_repo = repo.libgit2(&mut transaction).await?;
    let full_tree_name = branch.reference.name.as_bstr().to_str()?;

    let commit_ids = if let Some(ref path) = query.path {
        paged_commits_for_path(&libgit2_repo, full_tree_name, path, query.offset, limit).await?
    } else {
        paged_commits(&libgit2_repo, full_tree_name, query.offset, limit, reverse).await?
    };

    let mut commits = Vec::<FileCommitInfo>::with_capacity(commit_ids.len());

    for oid in commit_ids {
        let commit = libgit2_repo.find_commit(oid)?;
        let (author_name, author_uid, author_email) = commit.author().try_disassemble(&mut transaction).await;

        commits.push(FileCommitInfo {
            sha1: format!("{}", commit.id()),
            info: CommitInfo {
                message: commit.message().unwrap_or_default().to_owned(),
                time: commit.time().seconds(),
                author_name,
                author_email,
                author_uid,
            },
        });
    }

    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(BranchCommitsResponse { commits }))
}
