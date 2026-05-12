use crate::config::{get_optional_setting, get_setting};
use crate::organization::{OrgMember, Organization};
use crate::prelude::HttpRequestExtensions;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::replication::ImportTask;
use crate::repository::Repository;
use crate::routes::repository::api::{CreateJsonResponse, determine_namespace};
use crate::user::WebUser;
use crate::utils::identifiers::{is_fs_legal, is_reserved_repo_name, is_valid};
use crate::{die, err};
use gitarena_common::database::Pool;

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::{Context, Result};
use fang::{AsyncQueue, AsyncQueueable, AsyncRunnable};
use futures_locks::RwLock;
use gitarena_macros::route;
use serde::Deserialize;
use tracing::info;
use url::Url;
use utoipa::ToSchema;
use uuid::Uuid;

// todo: This whole handler is very similar to `create_repo.rs` so at some point this should be consolidated into one

#[utoipa::path(
    post,
    path = "/api/repo/import",
    request_body = ImportJsonRequest,
    responses(
        (status = 200, description = "Repository import queued successfully", body = CreateJsonResponse),
        (status = 400, description = "Invalid repository name, description, or import URL"),
        (status = 401, description = "Authentication required"),
        (status = 409, description = "Repository name already in use"),
        (status = 501, description = "Importing is disabled on this instance"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repo/import", method = "POST", err = "json")]
pub(crate) async fn import(
    web_user: WebUser,
    body: web::Json<ImportJsonRequest>,
    queue: web::Data<AsyncQueue>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut tx = db_pool.begin().await?;

    let enabled: bool = get_setting("repositories.importing_enabled", &mut tx).await?;

    if !enabled {
        die!(NOT_IMPLEMENTED, "Importing is disabled on this instance");
    }

    let name = &body.name;

    if name.is_empty() || name.len() > 32 || !name.chars().all(is_valid) {
        die!(
            BAD_REQUEST,
            "Repository name must be between 1 and 32 characters long and may only contain a-z, 0-9, _ or -"
        );
    }

    if is_reserved_repo_name(name.as_str()) {
        die!(BAD_REQUEST, "Repository name is a reserved identifier");
    }

    if !is_fs_legal(name) {
        die!(BAD_REQUEST, "Repository name is illegal");
    }

    let description = &body.description;

    if description.len() > 256 {
        die!(BAD_REQUEST, "Description may only be up to 256 characters long");
    }

    let url = Url::parse(body.import_url.as_str()).map_err(|_| err!(BAD_REQUEST, "Unable to parse import url"))?;
    let scheme = url.scheme();

    if scheme != "http" && scheme != "https" {
        die!(BAD_REQUEST, "Importing is only supported from `http` or `https` urls");
    }

    // as we print the url in our logs, disallow including credentials directly in it
    if !url.username().is_empty() || url.password().is_some() {
        die!(BAD_REQUEST, "Username and password is not allowed directly in the clone url");
    }

    if body.mirror.is_some() {
        die!(NOT_IMPLEMENTED, "Mirroring is not yet implemented");
    }

    let username = &body.username;
    let password = &body.password;

    if (username.is_some() && password.is_none()) || (username.is_none() && password.is_some()) {
        die!(BAD_REQUEST, "Either provide both username or password or leave both blank");
    }

    let (owner_id, owner_name) = determine_namespace(&body.namespace, &user, &mut tx).await?;
    let owner_col = if body.namespace == user.username { "owner_user" } else { "owner_org" };

    let (exists,): (bool,) = sqlx::query_as(&format!(
        "select exists(select 1 from repositories where {owner_col} = $1 and lower(name) = lower($2) limit 1)"
    ))
    .bind(owner_id)
    .bind(name)
    .fetch_one(&mut *tx)
    .await?;

    if exists {
        die!(CONFLICT, "Repository name already in use for this namespace");
    }

    let repo = sqlx::query_as::<_, Repository>(&format!(
        "insert into repositories (id, {owner_col}, name, description, visibility) values ($1, $2, $3, $4, $5) returning *"
    ))
    .bind(Uuid::now_v7())
    .bind(owner_id)
    .bind(name)
    .bind(description)
    .bind(body.visibility)
    .fetch_one(&mut *tx)
    .await?;

    repo.create_fs(&mut tx).await?;

    let task = ImportTask {
        source: url.to_string(),
        target: repo.clone(),
        username: username.clone(),
        password: password.clone(),
    };

    queue
        .insert_task(&task as &dyn AsyncRunnable)
        .await
        .context("failed to enqueue importing task")?;

    let domain: String = get_optional_setting("domain", &mut tx).await?.unwrap_or_default();

    tx.commit().await?;

    info!(
        target.id = %repo.id,
        target.owner = owner_name,
        target.name = repo.name,
        source.url = %url,
        "New repository created for importing",
    );

    Ok(HttpResponse::Ok().json(CreateJsonResponse {
        id: repo.id,
        url: format!("{domain}/{owner_name}/{}", repo.name),
    }))
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct ImportJsonRequest {
    /// Namespace (username or org name) under which the imported repo should be created
    namespace: String,
    /// Repository name
    #[schema(max_length = 32, pattern = "^[a-z0-9_-]+$")]
    name: String,
    /// Short description of the repository
    #[schema(max_length = 256)]
    description: String,
    /// URL of the remote repository to import
    #[serde(rename = "url")]
    #[schema(format = Uri)]
    import_url: String,
    /// Set to true to mirror the repository
    #[serde(default)]
    mirror: Option<bool>,
    /// Visibility of the imported repository
    visibility: RepoVisibility,

    /// Username for authenticating with the remote
    #[serde(default)]
    username: Option<String>,
    /// Password for authenticating with the remote
    #[serde(default)]
    password: Option<String>,
}
