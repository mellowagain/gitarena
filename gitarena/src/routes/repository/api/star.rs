use crate::die;
use crate::repository::Repository;
use crate::user::{User, WebUser};

use crate::database::Database;
use crate::database::Pool;
use crate::events::Event;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde::Serialize;
use sqlx::Transaction;
use tracing::{debug, instrument};
use utoipa::ToSchema;

/// Statistics for a repository
#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct RepoStatsStarsResponse {
    /// Size of the repository in bytes
    size: u64,
    /// Stars
    stars: RepoStatsDetailResponse,
    /// Forks
    forks: RepoStatsDetailResponse,
    /// Watchers
    watchers: RepoStatsDetailResponse,
}

/// Details about a statistic like Stars, Forks or Watchers
#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct RepoStatsDetailResponse {
    /// Total number
    count: i64,
    /// Whether the authenticated user has itself has given it for the repository
    #[serde(rename = "self")]
    self_starred: bool,
}

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/stats",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "Statistics about the repository", body = RepoStatsStarsResponse),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/stats", method = "GET", err = "json")]
pub(crate) async fn get_stats(repo: Repository, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let self_starred = if let Some(user) = web_user.as_ref() {
        has_star(user, &repo, &mut transaction).await?
    } else {
        false
    };

    let response = RepoStatsStarsResponse {
        size: repo.repo_size(&mut transaction).await?,
        stars: RepoStatsDetailResponse {
            count: get_star_count(&repo, &mut transaction).await?,
            self_starred,
        },
        forks: RepoStatsDetailResponse {
            count: get_fork_count(&repo, &web_user, &mut transaction).await?,
            self_starred: false, // TODO: implement self starred for forks
        },
        // TODO: implement watchers
        watchers: RepoStatsDetailResponse { count: 0, self_starred: false },
    };

    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(response))
}

#[utoipa::path(
    post,
    path = "/api/repo/{namespace}/{repository}/star",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
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
#[route("/api/repo/{namespace}/{repository}/star", method = "POST", err = "json")]
pub(crate) async fn post_star(repo: Repository, web_user: WebUser, request: HttpRequest, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut transaction = db_pool.begin().await?;

    if has_star(&user, &repo, &mut transaction).await? {
        die!(CONFLICT, "Already starred");
    }

    add_star(&user, &repo, &mut transaction).await?;

    Event::new("star.added", user.id, &request, (&repo).into(), None).save(&mut transaction).await?;

    transaction.commit().await?;

    Ok(HttpResponse::Created().finish())
}

#[utoipa::path(
    delete,
    path = "/api/repo/{namespace}/{repository}/star",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
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
#[route("/api/repo/{namespace}/{repository}/star", method = "DELETE", err = "json")]
pub(crate) async fn delete_star(repo: Repository, web_user: WebUser, request: HttpRequest, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut transaction = db_pool.begin().await?;

    if !has_star(&user, &repo, &mut transaction).await? {
        die!(CONFLICT, "Not starred");
    }

    remove_star(&user, &repo, &mut transaction).await?;

    Event::new("star.removed", user.id, &request, (&repo).into(), None)
        .save(&mut transaction)
        .await?;

    transaction.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

// not utopia annotated because its not a json api, but TODO we should change this
// TODO SLATED FOR DELETION
#[route("/api/repo/{namespace}/{repository}/star", method = "PUT", err = "text")]
pub(crate) async fn put_star(repo: Repository, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
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

#[instrument(err, skip(tx))]
pub(crate) async fn get_fork_count(repo: &Repository, web_user: &WebUser, tx: &mut Transaction<'_, Database>) -> Result<i64> {
    let visibility_filter = if matches!(web_user, WebUser::Authenticated(_)) {
        "visibility != 'private'"
    } else {
        "visibility = 'public'"
    };

    let query = format!("select count(*) from repositories where forked_from = $1 and disabled = false and {visibility_filter}");

    let (count,): (i64,) = sqlx::query_as(query.as_str()).bind(repo.id).fetch_optional(&mut **tx).await?.unwrap_or((0,));

    Ok(count)
}

#[instrument(err, skip(tx))]
async fn get_star_count(repo: &Repository, tx: &mut Transaction<'_, Database>) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as("select count(*) from stars where repo = $1")
        .bind(repo.id)
        .fetch_optional(&mut **tx)
        .await?
        .unwrap_or((0,));

    Ok(count)
}

#[instrument(err, skip(tx))]
async fn add_star(user: &User, repo: &Repository, tx: &mut Transaction<'_, Database>) -> Result<()> {
    sqlx::query("insert into stars (stargazer, repo) values ($1, $2)")
        .bind(user.id)
        .bind(repo.id)
        .execute(&mut **tx)
        .await?;

    debug!(user.username, user.id = %user.id, repo.id = %repo.id, "Star added to repo");

    Ok(())
}

#[instrument(err, skip(tx))]
async fn remove_star(user: &User, repo: &Repository, tx: &mut Transaction<'_, Database>) -> Result<()> {
    sqlx::query("delete from stars where stargazer = $1 and repo = $2")
        .bind(user.id)
        .bind(repo.id)
        .execute(&mut **tx)
        .await?;

    debug!(user.username, user.id = %user.id, repo.id = %repo.id, "Star removed from repo");

    Ok(())
}

#[instrument(err, skip(tx))]
async fn has_star(user: &User, repo: &Repository, tx: &mut Transaction<'_, Database>) -> Result<bool> {
    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from stars where stargazer = $1 and repo = $2 limit 1)")
        .bind(user.id)
        .bind(repo.id)
        .fetch_one(&mut **tx)
        .await?;

    Ok(exists)
}
