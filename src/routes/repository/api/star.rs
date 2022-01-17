use crate::prelude::HttpRequestExtensions;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;
use crate::user::{User, WebUser};
use crate::{die, err};

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use log::debug;
use serde_json::json;
use sqlx::{Executor, PgPool, Postgres};

#[route("/api/repo/{username}/{repository}/star", method = "GET", err = "htmx+json")]
pub(crate) async fn get_star(uri: web::Path<GitRequest>, web_user: WebUser, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let repo_owner = User::find_using_name(&uri.username, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;
    let repo = Repository::open(&repo_owner, &uri.repository, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;

    if !privilege::check_access(&repo, web_user.as_ref(), &mut transaction).await? {
        die!(NOT_FOUND, "Repository not found");
    }

    let count = get_star_count(&repo, &mut transaction).await?;

    let self_stargazer = if let Some(user) = web_user.as_ref() {
        has_star(user, &repo, &mut transaction).await?
    } else {
        false
    };

    transaction.commit().await?;

    if request.get_header("hx-request").is_some() {
        Ok(HttpResponse::Ok().body(count.to_string()))
    } else {
        Ok(HttpResponse::Ok().json(json!({
            "repo": format!("{}/{}", repo_owner.username.as_str(), repo.name.as_str()),
            "stars": count,
            "self": self_stargazer
        })))
    }
}

#[route("/api/repo/{username}/{repository}/star", method = "POST", err = "json")]
pub(crate) async fn post_star(uri: web::Path<GitRequest>, web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut transaction = db_pool.begin().await?;

    let repo_owner = User::find_using_name(&uri.username, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;
    let repo = Repository::open(repo_owner, &uri.repository, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;

    if !privilege::check_access(&repo, Some(&user), &mut transaction).await? {
        die!(NOT_FOUND, "Not found");
    }

    if has_star(&user, &repo, &mut transaction).await? {
        die!(CONFLICT, "Already starred");
    }

    add_star(&user, &repo, &mut transaction).await?;

    transaction.commit().await?;

    Ok(HttpResponse::Created().finish())
}

#[route("/api/repo/{username}/{repository}/star", method = "DELETE", err = "json")]
pub(crate) async fn delete_star(uri: web::Path<GitRequest>, web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut transaction = db_pool.begin().await?;

    let repo_owner = User::find_using_name(&uri.username, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;
    let repo = Repository::open(repo_owner, &uri.repository, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;

    if !privilege::check_access(&repo, Some(&user), &mut transaction).await? {
        die!(NOT_FOUND, "Not found");
    }

    if !has_star(&user, &repo, &mut transaction).await? {
        die!(CONFLICT, "Not starred");
    }

    remove_star(&user, &repo, &mut transaction).await?;

    transaction.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

#[route("/api/repo/{username}/{repository}/star", method = "PUT", err = "text")]
pub(crate) async fn put_star(uri: web::Path<GitRequest>, web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut transaction = db_pool.begin().await?;

    let repo_owner = User::find_using_name(&uri.username, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;
    let repo = Repository::open(repo_owner, &uri.repository, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;

    if !privilege::check_access(&repo, Some(&user), &mut transaction).await? {
        die!(NOT_FOUND, "Not found");
    }

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

async fn get_star_count<'e, E: Executor<'e, Database = Postgres>>(repo: &Repository, executor: E) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as("select count(*) from stars where repo = $1")
        .bind(repo.id)
        .fetch_optional(executor)
        .await?
        .unwrap_or((0,));

    Ok(count)
}

async fn add_star<'e, E: Executor<'e, Database = Postgres>>(user: &User, repo: &Repository, executor: E) -> Result<()> {
    sqlx::query("insert into stars (stargazer, repo) values ($1, $2)")
        .bind(user.id)
        .bind(repo.id)
        .execute(executor)
        .await?;

    debug!("{} (id {}) added a star to repository id {}", user.username, user.id, repo.id);

    Ok(())
}

async fn remove_star<'e, E: Executor<'e, Database = Postgres>>(user: &User, repo: &Repository, executor: E) -> Result<()> {
    sqlx::query("delete from stars where stargazer = $1 and repo = $2")
        .bind(user.id)
        .bind(repo.id)
        .execute(executor)
        .await?;

    debug!("{} (id {}) removed their star from repository id {}", user.username, user.id, repo.id);

    Ok(())
}

async fn has_star<'e, E: Executor<'e, Database = Postgres>>(user: &User, repo: &Repository, executor: E) -> Result<bool> {
    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from stars where stargazer = $1 and repo = $2 limit 1)")
        .bind(user.id)
        .bind(repo.id)
        .fetch_one(executor)
        .await?;

    Ok(exists)
}
