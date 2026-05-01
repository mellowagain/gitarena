use crate::git::history::all_commits;
use crate::repository::Repository;

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use git2::ErrorCode;
use gitarena_common::database::Pool;
use gitarena_macros::route;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BranchInfo {
    /// Branch name
    name: String,
    /// Total number of commits
    commit_count: usize,
    /// Number of commits the branch is ahead versus the default branch
    ahead: usize,
    /// Number of commits the branch is behind versus the default branch
    behind: usize,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct BranchesResponse {
    /// List of branches
    branches: Vec<BranchInfo>,
}

#[utoipa::path(
    get,
    path = "/api/repos/{username}/{repository}/branches",
    params(
        ("username" = String, Path, description = "Repository owner username"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "List of branches with commit counts and ahead/behind relative to the default branch", body = BranchesResponse),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{username}/{repository}/branches", method = "GET", err = "json")]
pub(crate) async fn branches(repo: Repository, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;
    let libgit2_repo = repo.libgit2(&mut transaction).await?;
    transaction.commit().await?;

    let default_ref = format!("refs/heads/{}", repo.default_branch);
    let default_oid = match libgit2_repo.find_reference(&default_ref) {
        Ok(r) => r.peel_to_commit()?.id(),
        Err(err) if err.code() == ErrorCode::NotFound => {
            return Ok(HttpResponse::Ok().json(BranchesResponse { branches: vec![] }));
        }
        Err(err) => return Err(err.into()),
    };

    let mut branches = Vec::<BranchInfo>::new();

    for reference in libgit2_repo.references()? {
        let reference = reference?;

        let Some(full_name) = reference.name() else { continue };
        let Some(short_name) = full_name.strip_prefix("refs/heads/") else { continue };

        let branch_oid = reference.peel_to_commit()?.id();
        let commit_count = all_commits(&libgit2_repo, full_name, 0).await?.len();
        let (ahead, behind) = libgit2_repo.graph_ahead_behind(branch_oid, default_oid)?;

        branches.push(BranchInfo {
            name: short_name.to_owned(),
            commit_count,
            ahead,
            behind,
        });
    }

    Ok(HttpResponse::Ok().json(BranchesResponse { branches }))
}
