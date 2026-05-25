use crate::database::Pool;
use crate::repository::Repository;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use chrono::{DateTime, Utc};
use gitarena_macros::route;
use serde::Serialize;
use sqlx::FromRow;
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

    let milestones: Vec<MilestoneEntry> =
        sqlx::query_as("select id, title, description, due_date, closed from milestones where repo_id = $1 order by created_at asc")
            .bind(repo.id)
            .fetch_all(&mut *tx)
            .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(MilestonesResponse { milestones }))
}

#[derive(Serialize, ToSchema, FromRow)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct MilestoneEntry {
    id: Uuid,
    title: String,
    description: Option<String>,
    due_date: Option<DateTime<Utc>>,
    closed: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct MilestonesResponse {
    milestones: Vec<MilestoneEntry>,
}
