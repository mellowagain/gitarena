use crate::database::{Database, Pool};
use crate::die;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::user::WebUser;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

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

    let labels: Vec<LabelEntry> = sqlx::query_as("select id, name, color, description from labels where repo_id = $1 order by name asc")
        .bind(repo.id)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(LabelsResponse { labels }))
}

#[utoipa::path(
    post,
    path = "/api/repos/{namespace}/{repository}/labels",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    request_body = CreateLabelRequest,
    responses(
        (status = 200, description = "Label created", body = LabelEntry),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Repository not found or access denied"),
        (status = 409, description = "Label name already exists"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/labels", method = "POST", err = "json")]
pub(crate) async fn create_label(repo: Repository, web_user: WebUser, body: web::Json<CreateLabelRequest>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    if !privilege::check_manage_issues(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let name = body.name.trim();
    if name.is_empty() {
        die!(BAD_REQUEST, "Label name cannot be empty");
    }

    let color_raw = body.color.trim().to_lowercase();
    let color = color_raw.as_str();
    if !is_valid_hex_color(color) {
        die!(BAD_REQUEST, "Color must be a valid hex color (e.g. #ff0000)");
    }

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from labels where repo_id = $1 and lower(name) = lower($2) limit 1)")
        .bind(repo.id)
        .bind(name)
        .fetch_one(&mut *tx)
        .await?;

    if exists {
        die!(CONFLICT, "A label with this name already exists");
    }

    let label: LabelEntry =
        sqlx::query_as("insert into labels (id, repo_id, name, color, description) values ($1, $2, $3, $4, $5) returning id, name, color, description")
            .bind(Uuid::now_v7())
            .bind(repo.id)
            .bind(name)
            .bind(color)
            .bind(body.description.as_deref().unwrap_or("").trim())
            .fetch_one(&mut *tx)
            .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(label))
}

#[utoipa::path(
    put,
    path = "/api/repos/{namespace}/{repository}/labels/{label_id}",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
        ("label_id" = String, Path, description = "Label UUID"),
    ),
    request_body = UpdateLabelRequest,
    responses(
        (status = 200, description = "Label updated", body = LabelEntry),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Repository or label not found"),
        (status = 409, description = "Label name already exists"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/labels/{label_id}", method = "PUT", err = "json")]
pub(crate) async fn update_label(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, Uuid)>,
    body: web::Json<UpdateLabelRequest>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;
    let (_, _, label_id) = path.into_inner();

    if !privilege::check_manage_issues(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let existing: Option<LabelEntry> = sqlx::query_as("select id, name, color, description from labels where id = $1 and repo_id = $2")
        .bind(label_id)
        .bind(repo.id)
        .fetch_optional(&mut *tx)
        .await?;

    let existing = match existing {
        Some(l) => l,
        None => die!(NOT_FOUND, "Label not found"),
    };

    let new_name = body.name.as_deref().map(str::trim).unwrap_or(&existing.name);
    let new_color_raw;
    let new_color = if let Some(c) = body.color.as_deref() {
        new_color_raw = c.trim().to_lowercase();
        new_color_raw.as_str()
    } else {
        existing.color.as_str()
    };
    let new_description = body
        .description
        .as_deref()
        .map(str::trim)
        .unwrap_or(existing.description.as_deref().unwrap_or(""));

    if !is_valid_hex_color(new_color) {
        die!(BAD_REQUEST, "Color must be a valid hex color (e.g. #ff0000)");
    }

    // If the name changed, check for conflicts and update issue_cache
    if !new_name.eq_ignore_ascii_case(&existing.name) {
        let (conflicts,): (bool,) = sqlx::query_as("select exists(select 1 from labels where repo_id = $1 and lower(name) = lower($2) and id != $3 limit 1)")
            .bind(repo.id)
            .bind(new_name)
            .bind(label_id)
            .fetch_one(&mut *tx)
            .await?;

        if conflicts {
            die!(CONFLICT, "A label with this name already exists");
        }

        // Update all issues that reference the old label name
        sqlx::query("update issue_cache set labels = array_replace(labels, $1, $2) where repo_id = $3 and $1 = any(labels)")
            .bind(&existing.name)
            .bind(new_name)
            .bind(repo.id)
            .execute(&mut *tx)
            .await?;
    }

    let updated: LabelEntry = sqlx::query_as("update labels set name = $1, color = $2, description = $3 where id = $4 returning id, name, color, description")
        .bind(new_name)
        .bind(new_color)
        .bind(new_description)
        .bind(label_id)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(updated))
}

#[utoipa::path(
    delete,
    path = "/api/repos/{namespace}/{repository}/labels/{label_id}",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
        ("label_id" = String, Path, description = "Label UUID"),
    ),
    responses(
        (status = 204, description = "Label deleted"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Repository or label not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/labels/{label_id}", method = "DELETE", err = "json")]
pub(crate) async fn delete_label(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, Uuid)>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;
    let (_, _, label_id) = path.into_inner();

    if !privilege::check_manage_issues(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let existing: Option<(String,)> = sqlx::query_as("select name from labels where id = $1 and repo_id = $2")
        .bind(label_id)
        .bind(repo.id)
        .fetch_optional(&mut *tx)
        .await?;

    let (label_name,) = match existing {
        Some(l) => l,
        None => die!(NOT_FOUND, "Label not found"),
    };

    // Remove the label from all issues that reference it
    sqlx::query("update issue_cache set labels = array_remove(labels, $1) where repo_id = $2 and $1 = any(labels)")
        .bind(&label_name)
        .bind(repo.id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("delete from labels where id = $1").bind(label_id).execute(&mut *tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

/// Seed default labels for a newly created repository.
pub(crate) async fn seed_default_labels(repo_id: Uuid, tx: &mut Transaction<'_, Database>) -> Result<()> {
    let labels: &[(&str, &str, Option<&str>)] = &[
        // type scope
        ("type::bug", "#5c6bc0", None),
        ("type::feature", "#5c6bc0", None),
        ("type::docs", "#5c6bc0", None),
        ("type::chore", "#5c6bc0", Some("Maintenance tasks: dependency updates, CI config, tooling")),
        ("type::refactor", "#5c6bc0", Some("Internal code restructuring with no functional change")),
        // status scope
        ("status::blocked", "#e53935", None),
        (
            "status::needs-info",
            "#f4511e",
            Some("Waiting on the issue reporter or a user for more information"),
        ),
        ("status::in-progress", "#00897b", None),
        // unscoped
        ("good first issue", "#2e7d32", None),
        ("help wanted", "#388e3c", None),
        ("duplicate", "#bdbdbd", None),
        ("wontfix", "#bdbdbd", Some("Acknowledged but won't be addressed, see comments for reasoning")),
        ("question", "#ab47bc", None),
        ("security", "#b71c1c", Some("Involves a potential security vulnerability")),
    ];

    for (name, color, description) in labels {
        sqlx::query("insert into labels (id, repo_id, name, color, description) values ($1, $2, $3, $4, $5)")
            .bind(Uuid::now_v7())
            .bind(repo_id)
            .bind(name)
            .bind(color)
            .bind(description)
            .execute(&mut **tx)
            .await?;
    }

    Ok(())
}

fn is_valid_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

#[derive(Serialize, ToSchema, FromRow)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LabelEntry {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) color: String,
    pub(crate) description: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LabelsResponse {
    labels: Vec<LabelEntry>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateLabelRequest {
    /// Label name (use `scope::value` for scoped labels)
    name: String,
    /// Hex color (e.g. `#ff0000`)
    color: String,
    /// Optional description
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateLabelRequest {
    /// New label name
    #[serde(default)]
    name: Option<String>,
    /// New hex color
    #[serde(default)]
    color: Option<String>,
    /// New description
    #[serde(default)]
    description: Option<String>,
}
