use crate::config::get_optional_setting;
use crate::database::Pool;
use crate::die;
use crate::repository::{RepoOwner, Repository};
use crate::routes::repository::api::{CreateJsonResponse, determine_namespace};
use crate::user::WebUser;
use crate::utils::filesystem::copy_dir_all;

use std::path::Path;

use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use anyhow::{Context, Result, anyhow};
use gitarena_macros::route;
use serde::Deserialize;
use tracing::info;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/api/repo/{namespace}/{repository}/fork",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    request_body = ForkJsonRequest,
    responses(
        (status = 200, description = "Fork created successfully", body = CreateJsonResponse),
        (status = 400, description = "Cannot fork into this namespace"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Repository not found or access denied"),
        (status = 409, description = "Repository name already in use in this namespace"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repo/{namespace}/{repository}/fork", method = "POST", err = "json")]
pub(crate) async fn create_fork(
    repo: Repository,
    web_user: WebUser,
    body: web::Json<ForkJsonRequest>,
    request: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut tx = db_pool.begin().await?;

    let (target_id, target_name) = if let Some(namespace) = &body.target_namespace {
        determine_namespace(namespace, &user, &mut tx).await?
    } else {
        (user.id, user.username.clone())
    };

    let repo_owner = repo.owner_user.or(repo.owner_org);

    if repo_owner == Some(target_id) {
        die!(BAD_REQUEST, "Cannot fork your own repository into the same namespace");
    }

    let is_org_fork = target_id != user.id;
    let owner_col = if is_org_fork { "owner_org" } else { "owner_user" };

    let (exists,): (bool,) = sqlx::query_as(&format!(
        "select exists(select 1 from repositories where {owner_col} = $1 and lower(name) = lower($2) limit 1)"
    ))
    .bind(target_id)
    .bind(&repo.name)
    .fetch_one(&mut *tx)
    .await?;

    if exists {
        die!(CONFLICT, "Repository name already in use in this namespace");
    }

    let new_repo = sqlx::query_as::<_, Repository>(&format!(
        "insert into repositories (id, {owner_col}, name, description, visibility, forked_from) values ($1, $2, $3, $4, $5, $6) returning *"
    ))
    .bind(Uuid::now_v7())
    .bind(target_id)
    .bind(&repo.name)
    .bind(&repo.description)
    .bind(repo.visibility)
    .bind(repo.id)
    .fetch_one(&mut *tx)
    .await?;

    let old_path = repo.get_fs_path(&mut tx).await?;
    let new_path = new_repo.get_fs_path(&mut tx).await?;

    // TODO: turn this into a task
    copy_dir_all(Path::new(old_path.as_str()), Path::new(new_path.as_str()))
        .await
        .context("Failed to copy repository")?;

    let domain: String = get_optional_setting("domain", &mut tx).await?.unwrap_or_default();

    tx.commit().await?;

    let extensions = request.extensions();
    let repo_owner = extensions.get::<RepoOwner>().ok_or_else(|| anyhow!("Failed to lookup repo owner"))?;

    info!(
        target.id = %new_repo.id,
        target.owner = target_name,
        target.name = new_repo.name,
        source.id = %repo.id,
        source.owner = %repo_owner,
        source.name = repo.name,
        "New repository forked"
    );

    Ok(HttpResponse::Ok().json(CreateJsonResponse {
        id: new_repo.id,
        url: format!("{domain}/{target_name}/{}", new_repo.name),
    }))
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct ForkJsonRequest {
    /// Namespace (username or org name) to fork the repository into.
    /// Defaults to the authenticated user if omitted.
    #[serde(default)]
    pub(crate) target_namespace: Option<String>,
}
