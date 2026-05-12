use crate::organization::Organization;
use crate::user::WebUser;
use crate::utils::identifiers::{is_fs_legal, is_reserved_repo_name, is_valid};

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_common::database::Pool;
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// Validates a candidate repository name and description for the given namespace.
/// Always returns 200 with per-field error messages so the frontend can display them
/// on the correct field. Also checks that the name is not already in use in the namespace.
#[utoipa::path(
    get,
    path = "/api/repo/validate",
    params(ValidateQuery),
    responses(
        (status = 200, description = "Validation result with optional per-field errors", body = ValidateResponse),
        (status = 401, description = "Authentication required"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repo/validate", method = "GET", err = "json")]
pub(crate) async fn validate(web_user: WebUser, query: web::Query<ValidateQuery>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let user = web_user.into_user()?;

    let name = &query.name;
    let mut name_error: Option<String> = None;
    let mut description_error: Option<String> = None;

    if name.is_empty() || name.len() > 32 || !name.chars().all(is_valid) {
        name_error = Some("Repository name must be between 1 and 32 characters long and may only contain a-z, 0-9, _ or -".to_string());
    } else if is_reserved_repo_name(name.as_str()) {
        name_error = Some("Repository name is a reserved identifier".to_string());
    } else if !is_fs_legal(name) {
        name_error = Some("Repository name is illegal".to_string());
    } else {
        // Determine the actual owner to check against.
        // If namespace matches the user's own username (or is absent), check user repos.
        // If namespace matches an org, check org repos.
        let effective_namespace = query.namespace.as_deref().unwrap_or(&user.username);

        let exists: bool = if effective_namespace == user.username {
            let (e,): (bool,) = sqlx::query_as("select exists(select 1 from repositories where owner_user = $1 and lower(name) = lower($2) limit 1)")
                .bind(user.id)
                .bind(name)
                .fetch_one(&mut *transaction)
                .await?;
            e
        } else if let Some(org) = Organization::find_by_name(effective_namespace, &mut transaction).await {
            let (e,): (bool,) = sqlx::query_as("select exists(select 1 from repositories where owner_org = $1 and lower(name) = lower($2) limit 1)")
                .bind(org.id)
                .bind(name)
                .fetch_one(&mut *transaction)
                .await?;
            e
        } else {
            // Namespace not found; duplicate check not meaningful, skip it
            false
        };

        if exists {
            name_error = Some("Repository name already in use for this namespace".to_string());
        }
    }

    if let Some(description) = &query.description
        && description.len() > 256
    {
        description_error = Some("Description may only be up to 256 characters long".to_string());
    }

    transaction.commit().await?;

    let valid = name_error.is_none() && description_error.is_none();

    Ok(HttpResponse::Ok().json(ValidateResponse {
        valid,
        name: name_error,
        description: description_error,
    }))
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub(crate) struct ValidateQuery {
    /// Namespace (username or org name) to check against; defaults to the authenticated user
    namespace: Option<String>,
    /// Candidate repository name
    name: String,
    /// Optional candidate description (validated for length if provided)
    description: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct ValidateResponse {
    /// Whether both name and description are valid
    valid: bool,
    /// Error message for the name field, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Error message for the description field, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}
