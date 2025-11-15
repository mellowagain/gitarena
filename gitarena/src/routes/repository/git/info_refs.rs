use crate::die;
use crate::git::basic_auth;
use crate::git::capabilities::capabilities;
use crate::git::ls_refs::ls_refs_all;
use crate::prelude::*;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;

use actix_web::http::header::CONTENT_TYPE;
use actix_web::{web, Either, HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use gitarena_macros::route;
use sqlx::{Executor, PgPool, Pool, Postgres};

#[route("/{username}/{repository}.git/info/refs", method = "GET", err = "text")]
pub(crate) async fn info_refs(
    uri: web::Path<GitRequest>,
    request: HttpRequest,
    db_pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let query_string = request.q_string();

    let service = match query_string.get("service") {
        Some(value) => value.trim(),
        None => die!(BAD_REQUEST, "Dumb clients are not supported"),
    };

    let mut transaction = db_pool.begin().await?;

    let user_option: Option<(i32,)> =
        sqlx::query_as("select id from users where lower(username) = lower($1) limit 1")
            .bind(&uri.username)
            .fetch_optional(&mut transaction)
            .await?;

    let (user_id,) = match user_option {
        Some(user_id) => user_id,
        None => die!(NOT_FOUND),
    };

    let repo_option: Option<Repository> = sqlx::query_as::<_, Repository>(
        "select * from repositories where owner = $1 and lower(name) = lower($2) limit 1",
    )
    .bind(user_id)
    .bind(&uri.repository)
    .fetch_optional(&mut transaction)
    .await?;

    match service {
        "git-upload-pack" => {
            let response =
                upload_pack_info_refs(repo_option, service, &request, &mut transaction).await?;
            transaction.commit().await?;

            Ok(response)
        }
        "git-receive-pack" => {
            let response = receive_pack_info_refs(repo_option, &request, &db_pool).await?;
            transaction.commit().await?;

            Ok(response)
        }
        _ => die!(FORBIDDEN, "Requested service not found"),
    }
}

async fn upload_pack_info_refs<'e, E>(
    repo_option: Option<Repository>,
    service: &str,
    request: &HttpRequest,
    executor: E,
) -> Result<HttpResponse>
where
    E: Executor<'e, Database = Postgres>,
{
    let git_protocol = request.get_header("git-protocol").unwrap_or_default();

    if git_protocol != "version=2" {
        die!(BAD_REQUEST, "Unsupported Git protocol version");
    }

    let (_, _) = match basic_auth::validate_repo_access(
        repo_option,
        "application/x-git-upload-pack-advertisement",
        request,
        executor,
    )
    .await?
    {
        Either::Left(tuple) => tuple,
        Either::Right(response) => return Ok(response),
    };

    Ok(HttpResponse::Ok()
        .append_header((CONTENT_TYPE, "application/x-git-upload-pack-advertisement"))
        .body(capabilities(service).await?))
}

async fn receive_pack_info_refs(
    repo_option: Option<Repository>,
    request: &HttpRequest,
    db_pool: &Pool<Postgres>,
) -> Result<HttpResponse> {
    let mut transaction = db_pool.begin().await?;

    let _user = match basic_auth::login_flow(
        request,
        &mut transaction,
        "application/x-git-receive-pack-advertisement",
    )
    .await?
    {
        Either::Left(user) => user,
        Either::Right(response) => return Ok(response),
    };

    // TODO: Check if the user has actually `write` access to the repository

    let repo = match repo_option {
        Some(repo) => repo,
        None => die!(NOT_FOUND),
    };

    let git2repo = repo.libgit2(&mut transaction).await?;
    let output = ls_refs_all(&git2repo).await?;

    transaction.commit().await?;

    Ok(HttpResponse::Ok()
        .append_header((CONTENT_TYPE, "application/x-git-receive-pack-advertisement"))
        .body(output))
}
