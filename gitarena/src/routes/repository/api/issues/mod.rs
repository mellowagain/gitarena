use crate::database::{Database, Pool};
use crate::die;
use crate::issue::{IssueCache, IssueCommentCache, IssueStatus};
use crate::meili::MeiliClient;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::user::WebUser;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use chrono::{DateTime, Utc};
use gitarena_issues::bug::{create_bug, delete_bug, load_bug};
use gitarena_issues::ops::{change_labels, edit_comment, set_status, set_title};
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Transaction};
use std::collections::HashSet;
use tracing::{info, instrument};
use utoipa::ToSchema;
use uuid::Uuid;

pub(crate) mod comments;
pub(crate) mod labels;
pub(crate) mod milestones;
pub(crate) mod reactions;
pub(crate) mod timeline;

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/issues",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "List of issues", body = Vec<IssueResponse>),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/issues", method = "GET", err = "json")]
pub(crate) async fn list_issues(repo: Repository, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut tx = db_pool.begin().await?;

    let show_confidential = web_user.as_ref().is_some_and(|user| repo.owner_user == Some(user.id));

    let issues: Vec<IssueCache> = if show_confidential {
        sqlx::query_as("select * from issue_cache where repo_id = $1 order by \"index\" desc")
            .bind(repo.id)
            .fetch_all(&mut *tx)
            .await?
    } else {
        sqlx::query_as("select * from issue_cache where repo_id = $1 and confidential = false order by \"index\" desc")
            .bind(repo.id)
            .fetch_all(&mut *tx)
            .await?
    };

    let mut responses = Vec::with_capacity(issues.len());

    for issue in &issues {
        responses.push(build_issue_response(issue, web_user.as_ref().map(|u| u.id), &mut tx).await?);
    }

    tx.commit().await?;

    let total = responses.len();
    Ok(HttpResponse::Ok().json(IssuesListResponse { issues: responses, total }))
}

#[utoipa::path(
    post,
    path = "/api/repos/{namespace}/{repository}/issues",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    request_body = CreateIssueRequest,
    responses(
        (status = 201, description = "Issue created", body = IssueResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/issues", method = "POST", err = "json")]
pub(crate) async fn create_issue(
    repo: Repository,
    web_user: WebUser,
    body: web::Json<CreateIssueRequest>,
    db_pool: web::Data<Pool>,
    meili_client: web::Data<MeiliClient>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    if body.title.trim().is_empty() {
        die!(BAD_REQUEST, "Issue title cannot be empty");
    }

    let mut tx = db_pool.begin().await?;

    let gitoxide_repo = repo.gitoxide(&mut tx).await?;
    let author = user.as_git_bug_author();

    let next_index: i32 = sqlx::query_scalar("update repositories set next_issue_index = next_issue_index + 1 where id = $1 returning next_issue_index")
        .bind(repo.id)
        .fetch_one(&mut *tx)
        .await?;

    let mut bug = create_bug(&gitoxide_repo, author.clone(), body.title.clone(), body.body.clone())?;

    if let Some(labels) = &body.labels
        && !labels.is_empty()
    {
        if let Some(scope) = conflicting_scope(labels) {
            die!(BAD_REQUEST, "Only one label per scope is allowed (conflict: {scope})");
        }

        bug = change_labels(&gitoxide_repo, bug, author, labels.clone(), vec![])?;
    }

    let issue_id = Uuid::now_v7();
    let priority = body.priority.as_deref().unwrap_or("none");

    sqlx::query(
        "insert into issue_cache (id, repo_id, git_bug_id, \"index\", author_id, title, body, status, priority, labels, assignees, milestone_id) \
         values ($1, $2, $3, $4, $5, $6, $7, 'open', $8, $9, $10, $11)",
    )
    .bind(issue_id)
    .bind(repo.id)
    .bind(&bug.id)
    .bind(next_index)
    .bind(user.id)
    .bind(&body.title)
    .bind(&body.body)
    .bind(priority)
    .bind(body.labels.as_deref().unwrap_or(&[]))
    .bind(body.assignees.as_deref().unwrap_or(&[]))
    .bind(body.milestone_id)
    .execute(&mut *tx)
    .await?;

    let issue = get_issue_by_index(repo.id, next_index, &mut tx).await?;
    let response = build_issue_response(&issue, Some(user.id), &mut tx).await?;

    tx.commit().await?;

    info!(%repo.id, %bug.id, index = next_index, "issue created");

    issue.index_meili(&meili_client).await;

    Ok(HttpResponse::Created().json(response))
}

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/issues/{index}",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
        ("index" = i32, Path, description = "Issue number"),
    ),
    responses(
        (status = 200, description = "Issue with comments", body = IssueDetailResponse),
        (status = 404, description = "Issue not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/issues/{index}", method = "GET", err = "json")]
pub(crate) async fn get_issue_detail(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, i32)>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let (_, _, index) = path.into_inner();

    let mut tx = db_pool.begin().await?;

    let issue = get_issue_by_index(repo.id, index, &mut tx).await?;

    let show_confidential = web_user.as_ref().is_some_and(|user| repo.owner_user == Some(user.id));

    if issue.confidential && !show_confidential {
        die!(NOT_FOUND, "Issue not found");
    }

    let viewer_id = web_user.as_ref().map(|user| user.id);

    let raw_comments: Vec<IssueCommentCache> = sqlx::query_as("select * from issue_comment_cache where issue_id = $1 order by id asc")
        .bind(issue.id)
        .fetch_all(&mut *tx)
        .await?;

    let mut comments = Vec::with_capacity(raw_comments.len());

    for comment in &raw_comments {
        let author_username = resolve_username(comment.author_id, &mut tx).await?;
        let reactions = load_reactions(ReactionTarget::Comment(comment.id), viewer_id, &mut tx).await?;

        comments.push(CommentResponse {
            id: comment.id,
            author_username,
            body: comment.body.clone(),
            edited_at: comment.edited_at,
            reactions,
        });
    }

    let issue_response = build_issue_response(&issue, viewer_id, &mut tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(IssueDetailResponse {
        issue: issue_response,
        comments,
    }))
}

#[utoipa::path(
    patch,
    path = "/api/repos/{namespace}/{repository}/issues/{index}",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
        ("index" = i32, Path, description = "Issue number"),
    ),
    request_body = UpdateIssueRequest,
    responses(
        (status = 200, description = "Updated issue", body = IssueResponse),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Issue not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/issues/{index}", method = "PATCH", err = "json")]
pub(crate) async fn update_issue(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, i32)>,
    body: web::Json<UpdateIssueRequest>,
    meili_client: web::Data<MeiliClient>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let (_, _, index) = path.into_inner();

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    let issue = get_issue_by_index(repo.id, index, &mut tx).await?;

    let can_manage = privilege::check_manage_issues(&repo, Some(&user), &mut tx).await?;

    if !can_manage && issue.author_id != user.id {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let gitoxide_repo = repo.gitoxide(&mut tx).await?;
    let author = user.as_git_bug_author();

    if let Some(new_title) = &body.title {
        if new_title.trim().is_empty() {
            die!(BAD_REQUEST, "Issue title cannot be empty");
        }

        let bug = load_bug(&gitoxide_repo, &issue.git_bug_id)?;
        set_title(&gitoxide_repo, bug, author.clone(), new_title.clone())?;

        sqlx::query("update issue_cache set title = $1, updated_at = now() where id = $2")
            .bind(new_title)
            .bind(issue.id)
            .execute(&mut *tx)
            .await?;
    }

    if let Some(new_body) = &body.body {
        let bug = load_bug(&gitoxide_repo, &issue.git_bug_id)?;
        let bug_id = bug.id.clone();

        edit_comment(&gitoxide_repo, bug, author.clone(), bug_id, new_body.clone())?;

        sqlx::query("update issue_cache set body = $1, updated_at = now() where id = $2")
            .bind(new_body)
            .bind(issue.id)
            .execute(&mut *tx)
            .await?;
    }

    if let Some(new_status) = &body.status {
        let bug = load_bug(&gitoxide_repo, &issue.git_bug_id)?;
        set_status(&gitoxide_repo, bug, author.clone(), new_status.to_git_bug_status())?;

        sqlx::query("update issue_cache set status = $1, updated_at = now() where id = $2")
            .bind(new_status)
            .bind(issue.id)
            .execute(&mut *tx)
            .await?;

        info!(repo_id = %repo.id, index, status = ?new_status, "issue status changed");
    }

    let add = body.labels_add.clone().unwrap_or_default();
    let remove = body.labels_remove.clone().unwrap_or_default();

    if !add.is_empty() || !remove.is_empty() {
        let resulting: Vec<_> = issue
            .labels
            .iter()
            .filter(|l| !remove.contains(l))
            .cloned()
            .chain(add.iter().cloned())
            .collect();

        if let Some(scope) = conflicting_scope(&resulting) {
            die!(BAD_REQUEST, "Only one label per scope is allowed (conflict: {scope})");
        }

        let bug = load_bug(&gitoxide_repo, &issue.git_bug_id)?;
        change_labels(&gitoxide_repo, bug, author.clone(), add.clone(), remove.clone())?;

        sqlx::query(
            "update issue_cache set labels = (\
            select array_agg(l) from unnest(labels) l where l <> all($1::text[])\
            ) || $2::text[], updated_at = now() where id = $3",
        )
        .bind(&remove)
        .bind(&add)
        .bind(issue.id)
        .execute(&mut *tx)
        .await?;
    }

    if can_manage {
        if let Some(confidential) = body.confidential {
            sqlx::query("update issue_cache set confidential = $1, updated_at = now() where id = $2")
                .bind(confidential)
                .bind(issue.id)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(locked) = body.locked {
            sqlx::query("update issue_cache set locked = $1, updated_at = now() where id = $2")
                .bind(locked)
                .bind(issue.id)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(ref new_assignees) = body.assignees {
            sqlx::query("update issue_cache set assignees = $1, updated_at = now() where id = $2")
                .bind(new_assignees)
                .bind(issue.id)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(ref new_priority) = body.priority {
            sqlx::query("update issue_cache set priority = $1, updated_at = now() where id = $2")
                .bind(new_priority)
                .bind(issue.id)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(new_milestone_id) = body.milestone_id {
            sqlx::query("update issue_cache set milestone_id = $1, updated_at = now() where id = $2")
                .bind(new_milestone_id)
                .bind(issue.id)
                .execute(&mut *tx)
                .await?;
        }
    }

    let updated_issue = get_issue_by_index(repo.id, index, &mut tx).await?;
    let response = build_issue_response(&updated_issue, Some(user.id), &mut tx).await?;

    tx.commit().await?;

    updated_issue.index_meili(&meili_client).await;

    Ok(HttpResponse::Ok().json(response))
}

#[utoipa::path(
    delete,
    path = "/api/repos/{namespace}/{repository}/issues/{index}",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
        ("index" = i32, Path, description = "Issue number"),
    ),
    responses(
        (status = 204, description = "Issue deleted"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Issue not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/issues/{index}", method = "DELETE", err = "json")]
pub(crate) async fn delete_issue(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, i32)>,
    meili_client: web::Data<MeiliClient>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let (_, _, index) = path.into_inner();

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    let issue = get_issue_by_index(repo.id, index, &mut tx).await?;

    if !privilege::check_manage_issues(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let gitoxide_repo = repo.gitoxide(&mut tx).await?;
    delete_bug(&gitoxide_repo, &issue.git_bug_id)?;

    sqlx::query("delete from issue_cache where id = $1").bind(issue.id).execute(&mut *tx).await?;

    tx.commit().await?;

    info!(%repo.id, index, "issue deleted");

    IssueCache::deindex_meili(issue.id, &meili_client).await;

    Ok(HttpResponse::NoContent().finish())
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct MilestoneBrief {
    id: Uuid,
    title: String,
    closed: bool,
    due_date: Option<DateTime<Utc>>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct IssueResponse {
    id: Uuid,
    index: i32,
    git_bug_id: String,
    title: String,
    body: String,
    status: IssueStatus,
    priority: String,
    labels: Vec<String>,
    author_username: String,
    assignees: Vec<String>,
    milestone: Option<MilestoneBrief>,
    comment_count: i32,
    reactions: Vec<ReactionGroup>,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct IssueDetailResponse {
    issue: IssueResponse,
    comments: Vec<CommentResponse>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct IssuesListResponse {
    issues: Vec<IssueResponse>,
    total: usize,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct CommentResponse {
    pub(crate) id: Uuid,
    pub(crate) author_username: String,
    pub(crate) body: String,
    pub(crate) edited_at: Option<DateTime<Utc>>,
    pub(crate) reactions: Vec<ReactionGroup>,
}

#[derive(FromRow, Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct ReactionGroup {
    emoji: String,
    count: i64,
    reacted: bool,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateIssueRequest {
    title: String,
    body: String,
    priority: Option<String>,
    labels: Option<Vec<String>>,
    assignees: Option<Vec<Uuid>>,
    milestone_id: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateIssueRequest {
    title: Option<String>,
    body: Option<String>,
    status: Option<IssueStatus>,
    priority: Option<String>,
    labels_add: Option<Vec<String>>,
    labels_remove: Option<Vec<String>>,
    confidential: Option<bool>,
    locked: Option<bool>,
    assignees: Option<Vec<Uuid>>,
    milestone_id: Option<Option<Uuid>>,
}

fn conflicting_scope(labels: &[String]) -> Option<String> {
    let mut seen = HashSet::new();

    for label in labels {
        if let Some((scope, _)) = label.split_once("::")
            && !seen.insert(scope)
        {
            return Some(scope.to_owned());
        }
    }

    None
}

async fn resolve_username(id: Uuid, tx: &mut Transaction<'_, Database>) -> Result<String> {
    Ok(sqlx::query_scalar("select username from users where id = $1 limit 1")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?)
}

#[instrument(err, skip(tx))]
pub(super) async fn get_issue_by_index(repo_id: Uuid, index: i32, tx: &mut Transaction<'_, Database>) -> Result<IssueCache> {
    let issue: Option<IssueCache> = sqlx::query_as(r#"select * from issue_cache where repo_id = $1 and "index" = $2 limit 1"#)
        .bind(repo_id)
        .bind(index)
        .fetch_optional(&mut **tx)
        .await?;

    match issue {
        Some(issue) => Ok(issue),
        None => die!(NOT_FOUND, "Issue not found"),
    }
}

async fn resolve_milestone_brief(id: Option<Uuid>, tx: &mut Transaction<'_, Database>) -> Result<Option<MilestoneBrief>> {
    let Some(id) = id else {
        return Ok(None);
    };

    let row: Option<(String, bool, Option<DateTime<Utc>>)> = sqlx::query_as("select title, closed, due_date from milestones where id = $1 limit 1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;

    Ok(row.map(|(title, closed, due_date)| MilestoneBrief { id, title, closed, due_date }))
}

#[derive(Debug)]
pub(super) enum ReactionTarget {
    Issue(Uuid),
    Comment(Uuid),
}

#[instrument(err, skip(tx))]
pub(super) async fn load_reactions(target: ReactionTarget, viewer_id: Option<Uuid>, tx: &mut Transaction<'_, Database>) -> Result<Vec<ReactionGroup>> {
    let (column, id) = match target {
        ReactionTarget::Issue(id) => ("issue_id", id),
        ReactionTarget::Comment(id) => ("comment_id", id),
    };

    let query = format!(
        "select emoji, count(*) as count, ($2::uuid is not null and bool_or(user_id = $2)) as reacted \
         from reactions where {column} = $1 group by emoji order by emoji"
    );

    Ok(sqlx::query_as(&query).bind(id).bind(viewer_id).fetch_all(&mut **tx).await?)
}

#[instrument(err, skip(tx))]
async fn build_issue_response(issue: &IssueCache, viewer_id: Option<Uuid>, tx: &mut Transaction<'_, Database>) -> Result<IssueResponse> {
    let author_username = resolve_username(issue.author_id, tx).await?;

    let mut assignees = Vec::with_capacity(issue.assignees.len());

    for &uid in &issue.assignees {
        assignees.push(resolve_username(uid, tx).await?);
    }

    let milestone = resolve_milestone_brief(issue.milestone_id, tx).await?;
    let reactions = load_reactions(ReactionTarget::Issue(issue.id), viewer_id, tx).await?;

    let comment_count: i64 = sqlx::query_scalar("select count(*) from issue_comment_cache where issue_id = $1")
        .bind(issue.id)
        .fetch_one(&mut **tx)
        .await?;

    Ok(IssueResponse {
        id: issue.id,
        index: issue.index,
        git_bug_id: issue.git_bug_id.clone(),
        title: issue.title.clone(),
        body: issue.body.clone(),
        status: issue.status.clone(),
        priority: issue.priority.clone(),
        labels: issue.labels.clone(),
        author_username,
        assignees,
        milestone,
        comment_count: comment_count as i32,
        reactions,
        updated_at: issue.updated_at,
    })
}
