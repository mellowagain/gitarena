use crate::die;
use crate::prelude::HttpRequestExtensions;
use crate::repository::{RepoOwner, Repository};
use crate::user::{User, WebUser};

use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use anyhow::{Result, anyhow};
use gitarena_common::database::Database;
use gitarena_macros::route;
use log::debug;
use serde::Serialize;
use sqlx::{PgPool, Transaction};
use utoipa::ToSchema;

/// Star count for a repository
#[derive(Serialize, ToSchema)]
pub(crate) struct StarInfoResponse {
    /// Repository identifier in `owner/name` format
    repo: String,
    /// Total number of stars
    stars: i64,
    /// Whether the authenticated user has starred this repository
    #[serde(rename = "self")]
    self_starred: bool,
}

#[utoipa::path(
    get,
    path = "/api/repo/{username}/{repository}/star",
    params(
        ("username" = String, Path, description = "Repository owner username"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "Star count and viewer state", body = StarInfoResponse),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repo/{username}/{repository}/star", method = "GET", err = "htmx+json")]
pub(crate) async fn get_star(repo: Repository, web_user: WebUser, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let count = get_star_count(&repo, &mut transaction).await?;

    let self_stargazer = if let Some(user) = web_user.as_ref() {
        has_star(user, &repo, &mut transaction).await?
    } else {
        false
    };

    let extensions = request.extensions();
    let repo_owner = extensions.get::<RepoOwner>().ok_or_else(|| anyhow!("Failed to lookup repo owner"))?;

    transaction.commit().await?;

    if request.is_htmx() {
        Ok(HttpResponse::Ok().body(count.to_string()))
    } else {
        Ok(HttpResponse::Ok().json(StarInfoResponse {
            repo: format!("{}/{}", repo_owner, repo.name.as_str()),
            stars: count,
            self_starred: self_stargazer,
        }))
    }
}

#[utoipa::path(
    post,
    path = "/api/repo/{username}/{repository}/star",
    params(
        ("username" = String, Path, description = "Repository owner username"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 201, description = "Star added successfully"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Repository not found or access denied"),
        (status = 409, description = "Repository already starred"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repo/{username}/{repository}/star", method = "POST", err = "json")]
pub(crate) async fn post_star(repo: Repository, web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut transaction = db_pool.begin().await?;

    if has_star(&user, &repo, &mut transaction).await? {
        die!(CONFLICT, "Already starred");
    }

    add_star(&user, &repo, &mut transaction).await?;

    transaction.commit().await?;

    Ok(HttpResponse::Created().finish())
}

#[utoipa::path(
    delete,
    path = "/api/repo/{username}/{repository}/star",
    params(
        ("username" = String, Path, description = "Repository owner username"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 204, description = "Star removed successfully"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Repository not found or access denied"),
        (status = 409, description = "Repository was not starred"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repo/{username}/{repository}/star", method = "DELETE", err = "json")]
pub(crate) async fn delete_star(repo: Repository, web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut transaction = db_pool.begin().await?;

    if !has_star(&user, &repo, &mut transaction).await? {
        die!(CONFLICT, "Not starred");
    }

    remove_star(&user, &repo, &mut transaction).await?;

    transaction.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

// not utopia annotated because its not a json api, but TODO we should change this
#[route("/api/repo/{username}/{repository}/star", method = "PUT", err = "text")]
pub(crate) async fn put_star(repo: Repository, web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut transaction = db_pool.begin().await?;

    let mut response = HttpResponse::Ok();

    if has_star(&user, &repo, &mut transaction).await? {
        remove_star(&user, &repo, &mut transaction).await?;
        response.append_header(("x-gitarena-action", "remove"));
    } else {
        add_star(&user, &repo, &mut transaction).await?;
        response.append_header(("x-gitarena-action", "add"));
    }

    let count = get_star_count(&repo, &mut transaction).await?;

    transaction.commit().await?;

    Ok(response.body(count.to_string()))
}

async fn get_star_count(repo: &Repository, tx: &mut Transaction<'_, Database>) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as("select count(*) from stars where repo = $1")
        .bind(repo.id)
        .fetch_optional(&mut **tx)
        .await?
        .unwrap_or((0,));

    Ok(count)
}

async fn add_star(user: &User, repo: &Repository, tx: &mut Transaction<'_, Database>) -> Result<()> {
    sqlx::query("insert into stars (stargazer, repo) values ($1, $2)")
        .bind(user.id)
        .bind(repo.id)
        .execute(&mut **tx)
        .await?;

    debug!("{} (id {}) added a star to repository id {}", user.username, user.id, repo.id);

    Ok(())
}

async fn remove_star(user: &User, repo: &Repository, tx: &mut Transaction<'_, Database>) -> Result<()> {
    sqlx::query("delete from stars where stargazer = $1 and repo = $2")
        .bind(user.id)
        .bind(repo.id)
        .execute(&mut **tx)
        .await?;

    debug!("{} (id {}) removed their star from repository id {}", user.username, user.id, repo.id);

    Ok(())
}

async fn has_star(user: &User, repo: &Repository, tx: &mut Transaction<'_, Database>) -> Result<bool> {
    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from stars where stargazer = $1 and repo = $2 limit 1)")
        .bind(user.id)
        .bind(repo.id)
        .fetch_one(&mut **tx)
        .await?;

    Ok(exists)
}
