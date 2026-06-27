use crate::database::{Database, Pool};
use crate::die;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::user::WebUser;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use chrono::{DateTime, Utc};
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/milestones",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "List of milestones", body = MilestonesResponse),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/milestones", method = "GET", err = "json")]
pub(crate) async fn list_milestones(repo: Repository, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut tx = db_pool.begin().await?;

    let milestones: Vec<MilestoneEntry> = sqlx::query_as(
        "select m.id, m.title, m.description, m.due_date, m.closed,
         count(ic.id) filter (where ic.status in ('open', 'in_progress')) as open_issues,
         count(ic.id) filter (where ic.status in ('completed', 'not_planned')) as closed_issues
         from milestones m
         left join issue_cache ic on ic.milestone_id = m.id
         where m.repo_id = $1
         group by m.id
         order by m.created_at asc",
    )
    .bind(repo.id)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(MilestonesResponse { milestones }))
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MilestonesResponse {
    pub(crate) milestones: Vec<MilestoneEntry>,
}

#[derive(Serialize, ToSchema, FromRow)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MilestoneEntry {
    /// ID
    pub(crate) id: Uuid,
    /// Title
    pub(crate) title: String,
    /// Description
    pub(crate) description: Option<String>,
    /// Due date
    pub(crate) due_date: Option<DateTime<Utc>>,
    /// Whether milestone is closed
    pub(crate) closed: bool,
    /// Amount of open issues (open, in progress)
    pub(crate) open_issues: i64,
    /// Amount of closed issues (completed, not planned)
    pub(crate) closed_issues: i64,
}

#[utoipa::path(
    post,
    path = "/api/repos/{namespace}/{repository}/milestones",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    request_body = CreateMilestoneRequest,
    responses(
        (status = 200, description = "Milestone created", body = MilestoneEntry),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/milestones", method = "POST", err = "json")]
pub(crate) async fn create_milestone(
    repo: Repository,
    web_user: WebUser,
    body: web::Json<CreateMilestoneRequest>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    if !privilege::check_manage_issues(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let title = body.title.trim();

    if title.is_empty() {
        die!(BAD_REQUEST, "Milestone title cannot be empty");
    }

    let id = Uuid::now_v7();

    sqlx::query("insert into milestones (id, repo_id, title, description, due_date) values ($1, $2, $3, $4, $5)")
        .bind(id)
        .bind(repo.id)
        .bind(title)
        .bind(body.description.as_deref().unwrap_or("").trim())
        .bind(body.due_date)
        .execute(&mut *tx)
        .await?;

    let milestone = fetch_milestone_by_id(id, repo.id, &mut tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(milestone))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateMilestoneRequest {
    /// Title
    title: String,
    /// Description
    #[serde(default)]
    description: Option<String>,
    /// Due date
    #[serde(default)]
    due_date: Option<DateTime<Utc>>,
}

#[utoipa::path(
    patch,
    path = "/api/repos/{namespace}/{repository}/milestones/{milestone_id}",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
        ("milestone_id" = String, Path, description = "Milestone UUID"),
    ),
    request_body = UpdateMilestoneRequest,
    responses(
        (status = 200, description = "Milestone updated", body = MilestoneEntry),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Repository or milestone not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/milestones/{milestone_id}", method = "PATCH", err = "json")]
pub(crate) async fn update_milestone(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, Uuid)>,
    body: web::Json<UpdateMilestoneRequest>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let (_, _, milestone_id) = path.into_inner();
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    if !privilege::check_manage_issues(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let Some((existing_title, existing_description, existing_due_date, existing_closed)): Option<(String, Option<String>, Option<DateTime<Utc>>, bool)> =
        sqlx::query_as("select title, description, due_date, closed from milestones where id = $1 and repo_id = $2")
            .bind(milestone_id)
            .bind(repo.id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        die!(NOT_FOUND, "Milestone not found");
    };

    let new_title = body.title.as_deref().map(str::trim).unwrap_or(&existing_title);

    if new_title.is_empty() {
        die!(BAD_REQUEST, "Milestone title cannot be empty");
    }

    let new_description = body
        .description
        .as_deref()
        .map(str::trim)
        .unwrap_or(existing_description.as_deref().unwrap_or(""));

    let new_due_date = if body.due_date.is_some() { body.due_date } else { existing_due_date };
    let new_closed = body.closed.unwrap_or(existing_closed);

    sqlx::query("update milestones set title = $1, description = $2, due_date = $3, closed = $4 where id = $5")
        .bind(new_title)
        .bind(new_description)
        .bind(new_due_date)
        .bind(new_closed)
        .bind(milestone_id)
        .execute(&mut *tx)
        .await?;

    let milestone = fetch_milestone_by_id(milestone_id, repo.id, &mut tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(milestone))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateMilestoneRequest {
    /// Title
    #[serde(default)]
    title: Option<String>,
    /// Description
    #[serde(default)]
    description: Option<String>,
    /// Due date (set to null to clear)
    #[serde(default)]
    due_date: Option<DateTime<Utc>>,
    /// Whether the milestone is closed
    #[serde(default)]
    closed: Option<bool>,
}

#[utoipa::path(
    delete,
    path = "/api/repos/{namespace}/{repository}/milestones/{milestone_id}",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
        ("milestone_id" = String, Path, description = "Milestone UUID"),
    ),
    responses(
        (status = 204, description = "Milestone deleted"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Repository or milestone not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/milestones/{milestone_id}", method = "DELETE", err = "json")]
pub(crate) async fn delete_milestone(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, Uuid)>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let (_, _, milestone_id) = path.into_inner();
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    if !privilege::check_manage_issues(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let exists: bool = sqlx::query_scalar("select exists(select 1 from milestones where id = $1 and repo_id = $2)")
        .bind(milestone_id)
        .bind(repo.id)
        .fetch_one(&mut *tx)
        .await?;

    if !exists {
        die!(NOT_FOUND, "Milestone not found");
    }

    sqlx::query("delete from milestones where id = $1").bind(milestone_id).execute(&mut *tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

async fn fetch_milestone_by_id(id: Uuid, repo_id: Uuid, tx: &mut Transaction<'_, Database>) -> Result<MilestoneEntry> {
    Ok(sqlx::query_as(
        "select m.id, m.title, m.description, m.due_date, m.closed,
         count(ic.id) filter (where ic.status in ('open', 'in_progress')) as open_issues,
         count(ic.id) filter (where ic.status in ('completed', 'not_planned')) as closed_issues
         from milestones m
         left join issue_cache ic on ic.milestone_id = m.id
         where m.id = $1 and m.repo_id = $2
         group by m.id",
    )
    .bind(id)
    .bind(repo_id)
    .fetch_one(&mut **tx)
    .await?)
}
