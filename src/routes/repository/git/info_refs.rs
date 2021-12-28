use crate::error::GAErrors::GitError;
use crate::git::basic_auth;
use crate::git::capabilities::capabilities;
use crate::git::ls_refs::ls_refs_all;
use crate::prelude::*;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;

use actix_web::{Either, HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use sqlx::{Executor, PgPool, Pool, Postgres};

#[route("/{username}/{repository}.git/info/refs", method="GET")]
pub(crate) async fn info_refs(uri: web::Path<GitRequest>, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let query_string = request.q_string();

    let service = match query_string.get("service") {
        Some(value) => value.trim(),
        None => return Err(GitError(400, Some("Dumb clients are not supported".to_owned())).into())
    };

    let mut transaction = db_pool.begin().await?;

    let user_option: Option<(i32,)> = sqlx::query_as("select id from users where lower(username) = lower($1) limit 1")
        .bind(&uri.username)
        .fetch_optional(&mut transaction)
        .await?;

    let (user_id,) = match user_option {
        Some(user_id) => user_id,
        None => return Err(GitError(404, None).into())
    };

    let repo_option: Option<Repository> = sqlx::query_as::<_, Repository>("select * from repositories where owner = $1 and lower(name) = lower($2) limit 1")
        .bind(user_id)
        .bind(&uri.repository)
        .fetch_optional(&mut transaction)
        .await?;

    match service {
        "git-upload-pack" => {
            let response = upload_pack_info_refs(repo_option, service, &request, &mut transaction).await?;
            transaction.commit().await?;

            Ok(response)
        }
        "git-receive-pack" => {
            let response = receive_pack_info_refs(repo_option, &request, &db_pool).await?;
            transaction.commit().await?;

            Ok(response)
        }
        _ => {
            Err(GitError(403, Some("Requested service not found".to_owned())).into())
        }
    }
}

async fn upload_pack_info_refs<'e, E>(repo_option: Option<Repository>, service: &str, request: &HttpRequest, executor: E) -> Result<HttpResponse>
    where E: Executor<'e, Database = Postgres>
{
    let git_protocol = request.get_header("git-protocol").unwrap_or_default();

    if git_protocol != "version=2" {
        return Err(GitError(400, Some("Unsupported Git protocol version".to_owned())).into());
    }

    let (_, _) = match basic_auth::validate_repo_access(repo_option, "application/x-git-upload-pack-advertisement", request, executor).await? {
        Either::A(tuple) => tuple,
        Either::B(response) => return Ok(response)
    };

    Ok(HttpResponse::Ok()
        .header("Content-Type", "application/x-git-upload-pack-advertisement")
        .body(capabilities(service).await?))
}

async fn receive_pack_info_refs(repo_option: Option<Repository>, request: &HttpRequest, db_pool: &Pool<Postgres>) -> Result<HttpResponse> {
    let mut transaction = db_pool.begin().await?;

    let _user = match basic_auth::login_flow(request, &mut transaction, "application/x-git-receive-pack-advertisement").await? {
        Either::A(user) => user,
        Either::B(response) => return Ok(response)
    };

    // TODO: Check if the user has actually `write` access to the repository

    let repo = match repo_option {
        Some(repo) => repo,
        None => return Err(GitError(404, None).into())
    };

    let git2repo = repo.libgit2(&mut transaction).await?;
    let output = ls_refs_all(&git2repo).await?;

    transaction.commit().await?;

    Ok(HttpResponse::Ok()
        .header("Content-Type", "application/x-git-receive-pack-advertisement")
        .body(output))
}
