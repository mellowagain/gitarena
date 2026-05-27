use crate::database::Pool;
use crate::issue::IssueStatus;
use crate::user::WebUser;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use chrono::{DateTime, Utc};
use gitarena_macros::route;
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(FromRow)]
struct AssignedIssueRow {
    id: Uuid,
    index: i32,
    title: String,
    status: IssueStatus,
    priority: String,
    updated_at: DateTime<Utc>,
    repo_name: String,
    repo_namespace: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssignedIssueEntry {
    id: Uuid,
    index: i32,
    title: String,
    status: IssueStatus,
    priority: String,
    updated_at: DateTime<Utc>,
    repo_name: String,
    repo_namespace: String,
}

#[utoipa::path(
    get,
    path = "/api/users/me/assigned-issues",
    responses(
        (status = 200, description = "Issues assigned to the authenticated user", body = Vec<AssignedIssueEntry>),
        (status = 401, description = "Authentication required"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/users/me/assigned-issues", method = "GET", err = "json")]
pub(crate) async fn get_assigned_issues(web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let mut tx = db_pool.begin().await?;

    let rows: Vec<AssignedIssueRow> = sqlx::query_as(
        "select ic.id, ic.\"index\", ic.title, ic.status, ic.priority, ic.updated_at, \
         r.name as repo_name, coalesce(u.username, o.name, '') as repo_namespace \
         from issue_cache ic \
         join repositories r on ic.repo_id = r.id \
         left join users u on r.owner_user = u.id \
         left join organizations o on r.owner_org = o.id \
         where $1 = any(ic.assignees) \
         and ic.status in ('open', 'in_progress') \
         and ic.confidential = false \
         order by ic.updated_at desc \
         limit 20",
    )
    .bind(user.id)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    let issues: Vec<AssignedIssueEntry> = rows
        .into_iter()
        .map(|row| AssignedIssueEntry {
            id: row.id,
            index: row.index,
            title: row.title,
            status: row.status,
            priority: row.priority,
            updated_at: row.updated_at,
            repo_name: row.repo_name,
            repo_namespace: row.repo_namespace,
        })
        .collect();

    Ok(HttpResponse::Ok().json(issues))
}
