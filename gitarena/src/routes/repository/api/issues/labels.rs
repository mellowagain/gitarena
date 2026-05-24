use crate::database::Pool;
use crate::repository::Repository;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/labels",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "List of labels", body = LabelsResponse),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/labels", method = "GET", err = "json")]
pub(crate) async fn list_labels(repo: Repository, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut tx = db_pool.begin().await?;

    let labels: Vec<LabelEntry> = sqlx::query_as("select name, color from labels where repo_id = $1 order by name asc")
        .bind(repo.id)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(LabelsResponse { labels }))
}

#[derive(Serialize, ToSchema, FromRow)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct LabelEntry {
    name: String,
    color: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct LabelsResponse {
    labels: Vec<LabelEntry>,
}
