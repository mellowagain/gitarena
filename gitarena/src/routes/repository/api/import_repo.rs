use crate::config::{get_optional_setting, get_setting};
use crate::organization::{OrgMember, Organization};
use crate::prelude::HttpRequestExtensions;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::routes::repository::api::CreateJsonResponse;
use crate::user::WebUser;
use crate::utils::identifiers::{is_fs_legal, is_reserved_repo_name, is_valid};
use crate::{Ipc, die, err};
use gitarena_common::database::Pool;

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::{Context, Result};
use futures_locks::RwLock;
use gitarena_common::packets::git::GitImport;
use gitarena_macros::route;
use serde::Deserialize;
use tracing::info;
use url::Url;
use utoipa::ToSchema;
use uuid::Uuid;

// This whole handler is very similar to `create_repo.rs` so at some point this should be consolidated into one

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
    request: HttpRequest,
    ipc: web::Data<RwLock<Ipc>>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let mut transaction = db_pool.begin().await?;

    let enabled: bool = get_setting("repositories.importing_enabled", &mut transaction).await?;

    if !enabled || !ipc.read().await.is_connected() {
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

    if body.mirror.is_some() {
        die!(NOT_IMPLEMENTED, "Mirroring is not yet implemented");
    }

    // Resolve namespace to either the authenticated user or an org the user has access to
    let (owner_id, owner_name) = if body.namespace == user.username {
        (user.id, user.username.clone())
    } else if let Some(org) = Organization::find_by_name(&body.namespace, &mut transaction).await {
        if OrgMember::get_role(org.id, user.id, &mut transaction).await?.is_none() {
            die!(FORBIDDEN, "You are not a member of this organization");
        }
        (org.id, org.name.clone())
    } else {
        die!(BAD_REQUEST, "Namespace not found or you do not have access to it");
    };

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from repositories where owner = $1 and lower(name) = lower($2) limit 1)")
        .bind(owner_id)
        .bind(name)
        .fetch_one(&mut *transaction)
        .await?;

    if exists {
        die!(CONFLICT, "Repository name already in use for this namespace");
    }

    let repo: Repository =
        sqlx::query_as::<_, Repository>("insert into repositories (id, owner, name, description, visibility) values ($1, $2, $3, $4, $5) returning *")
            .bind(Uuid::now_v7())
            .bind(owner_id)
            .bind(name)
            .bind(description)
            .bind(body.visibility)
            .fetch_one(&mut *transaction)
            .await?;

    repo.create_fs(&mut transaction).await?;

    // Currently, only Git importing is supported. TODO: Support other VCS as well as GitLab export
    // At some point it is also planned to import issues and such, requiring support for specific hosters such as GitHub, GitLab, BitBucket and Gitea
    let packet = GitImport {
        url: url.to_string(),
        username: body.username.clone(),
        password: body.password.clone(),
    };

    ipc.write().await.send(packet).await.context("Failed to send import packet to workhorse")?;

    let domain: String = get_optional_setting("domain", &mut transaction).await?.unwrap_or_default();
    let path = format!("/{}/{}", &owner_name, &repo.name);

    transaction.commit().await?;

    info!(
        target.id = %repo.id,
        target.owner = owner_name,
        target.name = repo.name,
        source.url = %url,
        "New repository created for importing",
    );

    Ok(if request.is_htmx() {
        HttpResponse::Ok()
            .append_header(("hx-redirect", path))
            .append_header(("hx-refresh", "true"))
            .finish()
    } else {
        let url = format!("{domain}{path}");

        HttpResponse::Ok().json(CreateJsonResponse { id: repo.id, url })
    })
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
    /// Set to any value to mirror the repository
    #[serde(default)]
    mirror: Option<String>,
    /// Visibility of the imported repository
    visibility: RepoVisibility,

    /// Username for authenticating with the remote
    #[serde(default)]
    username: Option<String>,
    /// Password for authenticating with the remote
    #[serde(default)]
    password: Option<String>,
}
