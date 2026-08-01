use crate::database::Pool;
use crate::die;
use crate::events::Event;
use crate::issue::IssueCommentCache;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::api::issues::{CommentResponse, get_issue_by_index, resolve_username};
use crate::user::WebUser;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use chrono::Utc;
use gitarena_issues::bug::load_bug;
use gitarena_issues::ops::{add_comment, edit_comment};
use gitarena_macros::route;
use serde::Deserialize;
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/api/repos/{namespace}/{repository}/issues/{index}/comments",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
        ("index" = i32, Path, description = "Issue number"),
    ),
    request_body = AddCommentRequest,
    responses(
        (status = 201, description = "Comment added", body = CommentResponse),
        (status = 400, description = "Issue is locked"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Issue not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/issues/{index}/comments", method = "POST", err = "json")]
pub(crate) async fn add_issue_comment(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, i32)>,
    body: web::Json<AddCommentRequest>,
    request: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let (_, _, index) = path.into_inner();

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    let issue = get_issue_by_index(repo.id, index, &mut tx).await?;

    if issue.locked {
        die!(BAD_REQUEST, "Issue is locked");
    }

    let git_repo = repo.gitoxide(&mut tx).await?;
    let bug = load_bug(&git_repo, &issue.git_bug_id)?;

    let (_, op_id) = add_comment(&git_repo, bug, user.as_git_bug_author(), body.body.clone())?;
    let comment_id = Uuid::now_v7();

    sqlx::query("insert into issue_comment_cache (id, op_id, issue_id, author_id, body) values ($1, $2, $3, $4, $5)")
        .bind(comment_id)
        .bind(&op_id)
        .bind(issue.id)
        .bind(user.id)
        .bind(&body.body)
        .execute(&mut *tx)
        .await?;

    Event::new(
        "issue.comment_added",
        user.id,
        &request,
        (&repo).into(),
        Some(json!({
            "index": index,
            "body": body.body
        })),
    )
    .save(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Created().json(CommentResponse {
        id: comment_id,
        author_id: user.id,
        author_username: user.username,
        body: body.into_inner().body,
        edited_at: None,
        reactions: vec![],
    }))
}

#[utoipa::path(
    patch,
    path = "/api/repos/{namespace}/{repository}/issues/{index}/comments/{comment_id}",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
        ("index" = i32, Path, description = "Issue number"),
        ("comment_id" = Uuid, Path, description = "Comment ID"),
    ),
    request_body = EditCommentRequest,
    responses(
        (status = 200, description = "Comment updated", body = CommentResponse),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Comment not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/issues/{index}/comments/{comment_id}", method = "PATCH", err = "json")]
pub(crate) async fn edit_issue_comment(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, i32, Uuid)>,
    body: web::Json<EditCommentRequest>,
    request: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let (_, _, index, comment_id) = path.into_inner();

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    let issue = get_issue_by_index(repo.id, index, &mut tx).await?;

    let comment: IssueCommentCache = sqlx::query_as("select * from issue_comment_cache where id = $1 and issue_id = $2 limit 1")
        .bind(comment_id)
        .bind(issue.id)
        .fetch_one(&mut *tx)
        .await?;

    if comment.author_id != user.id && !privilege::check_manage_issues(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let author_username = resolve_username(comment.author_id, &mut tx).await?;

    let gitoxide_repo = repo.gitoxide(&mut tx).await?;

    let author = user.as_git_bug_author();
    let bug = load_bug(&gitoxide_repo, &issue.git_bug_id)?;

    edit_comment(&gitoxide_repo, bug, author, comment.op_id.clone(), body.body.clone())?;

    let now = Utc::now();

    sqlx::query("update issue_comment_cache set body = $1, edited_at = $2 where id = $3")
        .bind(&body.body)
        .bind(now)
        .bind(comment_id)
        .execute(&mut *tx)
        .await?;

    Event::new(
        "issue.comment_updated",
        user.id,
        &request,
        (&repo).into(),
        Some(json!({
            "index": index,
            "from": comment.body,
            "to": body.body
        })),
    )
    .save(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(CommentResponse {
        id: comment_id,
        author_id: comment.author_id,
        author_username,
        body: body.into_inner().body,
        edited_at: Some(now),
        reactions: vec![],
    }))
}

#[utoipa::path(
    delete,
    path = "/api/repos/{namespace}/{repository}/issues/{index}/comments/{comment_id}",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
        ("index" = i32, Path, description = "Issue number"),
        ("comment_id" = Uuid, Path, description = "Comment ID"),
    ),
    responses(
        (status = 204, description = "Comment deleted"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Comment not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/issues/{index}/comments/{comment_id}", method = "DELETE", err = "json")]
pub(crate) async fn delete_issue_comment(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, i32, Uuid)>,
    request: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let (_, _, index, comment_id) = path.into_inner();

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    let issue = get_issue_by_index(repo.id, index, &mut tx).await?;

    let comment: IssueCommentCache = sqlx::query_as("select * from issue_comment_cache where id = $1 and issue_id = $2 limit 1")
        .bind(comment_id)
        .bind(issue.id)
        .fetch_one(&mut *tx)
        .await?;

    if comment.author_id != user.id && !privilege::check_manage_issues(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    sqlx::query("delete from issue_comment_cache where id = $1")
        .bind(comment_id)
        .execute(&mut *tx)
        .await?;

    Event::new(
        "issue.comment_deleted",
        user.id,
        &request,
        (&repo).into(),
        Some(json!({
            "index": index,
            "comment_author": comment.author_id,
            "body": comment.body
        })),
    )
    .save(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddCommentRequest {
    body: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditCommentRequest {
    body: String,
}
