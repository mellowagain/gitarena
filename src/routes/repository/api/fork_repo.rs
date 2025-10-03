use crate::config::get_optional_setting;
use crate::die;
use crate::prelude::HttpRequestExtensions;
use crate::repository::{RepoOwner, Repository};
use crate::routes::repository::api::CreateJsonResponse;
use crate::user::WebUser;
use crate::utils::filesystem::copy_dir_all;

use std::path::Path;

use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use anyhow::{anyhow, Context, Result};
use gitarena_macros::route;
use log::info;
use serde_json::json;
use sqlx::PgPool;

#[route(
    "/api/repo/{username}/{repository}/fork",
    method = "GET",
    err = "htmx+json"
)]
pub(crate) async fn get_fork_amount(
    repo: Repository,
    web_user: WebUser,
    request: HttpRequest,
    db_pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let additional_query = if matches!(web_user, WebUser::Authenticated(_)) {
        // Allow public and unlisted repositories if the user is logged in
        "visibility != 'private'"
    } else {
        // Only allow public repositories, not unlisted or private repositories
        "visibility = 'public'"
    };

    let query = format!(
        "select count(*) from repositories where forked_from = $1 and disabled = false and {}",
        additional_query
    );

    let (count,): (i64,) = sqlx::query_as(query.as_str())
        .bind(repo.id)
        .fetch_optional(&mut transaction)
        .await?
        .unwrap_or((0,));

    transaction.commit().await?;

    if request.is_htmx() {
        Ok(HttpResponse::Ok().body(count.to_string()))
    } else {
        Ok(HttpResponse::Ok().json(json!({
            "forks": count
        })))
    }
}

#[route(
    "/api/repo/{username}/{repository}/fork",
    method = "POST",
    err = "htmx+text"
)]
pub(crate) async fn create_fork(
    repo: Repository,
    web_user: WebUser,
    request: HttpRequest,
    db_pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut transaction = db_pool.begin().await?;

    if repo.owner == user.id {
        die!(BAD_REQUEST, "Cannot fork your own repository");
    }

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from repositories where owner = $1 and lower(name) = lower($2) limit 1)")
        .bind(&user.id)
        .bind(&repo.name)
        .fetch_one(&mut transaction)
        .await?;

    if exists {
        die!(CONFLICT, "Repository name already in use for your account");
    }

    let new_repo = sqlx::query_as::<_, Repository>("insert into repositories (owner, name, description, visibility, forked_from) values ($1, $2, $3, $4, $5) returning *")
        .bind(&user.id)
        .bind(&repo.name)
        .bind(&repo.description)
        .bind(&repo.visibility)
        .bind(&repo.id)
        .fetch_one(&mut transaction)
        .await?;

    let old_path = repo.get_fs_path(&mut transaction).await?;
    let new_path = new_repo.get_fs_path(&mut transaction).await?;

    copy_dir_all(Path::new(old_path.as_str()), Path::new(new_path.as_str()))
        .await
        .context("Failed to copy repository")?;

    let domain = get_optional_setting::<String, _>("domain", &mut transaction)
        .await?
        .unwrap_or_default();
    let url = format!("{}/{}/{}", domain, user.username, new_repo.name);

    transaction.commit().await?;

    let extensions = request.extensions();
    let repo_owner = extensions
        .get::<RepoOwner>()
        .ok_or_else(|| anyhow!("Failed to lookup repo owner"))?;

    info!(
        "New repository forked: {}/{} (id {}, from {}/{})",
        &user.username, &new_repo.name, &repo.id, repo_owner, &repo.name
    );

    Ok(if request.is_htmx() {
        HttpResponse::Ok()
            .append_header(("hx-redirect", url))
            .append_header(("hx-refresh", "true"))
            .finish()
    } else {
        // TODO: Move CreateJsonResponse into mod.rs so it's no longer living inside of create_repo.rs
        HttpResponse::Ok().json(CreateJsonResponse {
            id: new_repo.id,
            url,
        })
    })
}
