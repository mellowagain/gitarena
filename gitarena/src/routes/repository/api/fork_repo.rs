use crate::config::get_optional_setting;
use crate::die;
use crate::prelude::HttpRequestExtensions;
use crate::repository::{RepoOwner, Repository};
use crate::routes::repository::api::CreateJsonResponse;
use crate::user::WebUser;
use crate::utils::filesystem::copy_dir_all;
use gitarena_common::database::Pool;

use std::path::Path;

use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use anyhow::{Context, Result, anyhow};
use gitarena_macros::route;
use tracing::info;

#[utoipa::path(
    post,
    path = "/api/repo/{username}/{repository}/fork",
    params(
        ("username" = String, Path, description = "Repository owner username"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "Fork created successfully", body = CreateJsonResponse),
        (status = 400, description = "Cannot fork your own repository"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Repository not found or access denied"),
        (status = 409, description = "Repository name already in use for your account"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repo/{username}/{repository}/fork", method = "POST", err = "json")]
pub(crate) async fn create_fork(repo: Repository, web_user: WebUser, request: HttpRequest, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut transaction = db_pool.begin().await?;

    if repo.owner == user.id {
        die!(BAD_REQUEST, "Cannot fork your own repository");
    }

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from repositories where owner = $1 and lower(name) = lower($2) limit 1)")
        .bind(user.id)
        .bind(&repo.name)
        .fetch_one(&mut *transaction)
        .await?;

    if exists {
        die!(CONFLICT, "Repository name already in use for your account");
    }

    let new_repo =
        sqlx::query_as::<_, Repository>("insert into repositories (owner, name, description, visibility, forked_from) values ($1, $2, $3, $4, $5) returning *")
            .bind(user.id)
            .bind(&repo.name)
            .bind(&repo.description)
            .bind(repo.visibility)
            .bind(repo.id)
            .fetch_one(&mut *transaction)
            .await?;

    let old_path = repo.get_fs_path(&mut transaction).await?;
    let new_path = new_repo.get_fs_path(&mut transaction).await?;

    copy_dir_all(Path::new(old_path.as_str()), Path::new(new_path.as_str()))
        .await
        .context("Failed to copy repository")?;

    let domain: String = get_optional_setting("domain", &mut transaction).await?.unwrap_or_default();
    let url = format!("{}/{}/{}", domain, user.username, new_repo.name);

    transaction.commit().await?;

    let extensions = request.extensions();
    let repo_owner = extensions.get::<RepoOwner>().ok_or_else(|| anyhow!("Failed to lookup repo owner"))?;

    info!(
        target.id = new_repo.id,
        target.owner = user.username,
        target.name = new_repo.name,
        source.id = repo.id,
        source.owner = %repo_owner,
        source.name = repo.name,
        "New repository forked"
    );

    Ok(
        // TODO: Move CreateJsonResponse into mod.rs so it's no longer living inside of create_repo.rs
        HttpResponse::Ok().json(CreateJsonResponse { id: new_repo.id, url }),
    )
}
