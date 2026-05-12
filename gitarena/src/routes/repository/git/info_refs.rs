use crate::die;
use crate::git::basic_auth;
use crate::git::capabilities::capabilities;
use crate::git::ls_refs::ls_refs_all;
use crate::organization::Organization;
use crate::prelude::*;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;
use crate::user::User;

use crate::privileges::privilege;
use actix_web::http::header::CONTENT_TYPE;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use either::Either;
use gitarena_common::database::Pool;
use gitarena_macros::route;
use tracing::instrument;
use uuid::Uuid;

#[route("/{namespace}/{repository}.git/info/refs", method = "GET", err = "text")]
pub(crate) async fn info_refs(uri: web::Path<GitRequest>, request: HttpRequest, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let query_string = request.q_string();

    let service = match query_string.get("service") {
        Some(value) => value.trim(),
        None => die!(BAD_REQUEST, "Dumb clients are not supported"),
    };

    let mut transaction = db_pool.begin().await?;

    let owner_id = resolve_namespace(&uri.namespace, &mut transaction).await?;
    let repo_option: Option<Repository> = Repository::open(owner_id, &uri.repository, &mut transaction).await;

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
        _ => die!(FORBIDDEN, "Requested service not found"),
    }
}

/// Resolve a namespace string to the UUID of the owning user or organization.
pub(crate) async fn resolve_namespace(namespace: &str, tx: &mut sqlx::Transaction<'_, gitarena_common::database::Database>) -> Result<Uuid> {
    if let Some(user) = User::find_using_name(namespace, tx).await {
        return Ok(user.id);
    }
    if let Some(org) = Organization::find_by_name(namespace, tx).await {
        return Ok(org.id);
    }
    die!(BAD_REQUEST, "Repository not found")
}

#[instrument(err, skip(request, tx))]
async fn upload_pack_info_refs(
    repo_option: Option<Repository>,
    service: &str,
    request: &HttpRequest,
    tx: &mut sqlx::Transaction<'_, gitarena_common::database::Database>,
) -> Result<HttpResponse> {
    let git_protocol = request.get_header("git-protocol").unwrap_or_default();

    if git_protocol != "version=2" {
        die!(BAD_REQUEST, "Unsupported Git protocol version");
    }

    let (_, _) = match basic_auth::validate_repo_access(repo_option, "application/x-git-upload-pack-advertisement", request, tx).await? {
        Either::Left(tuple) => tuple,
        Either::Right(response) => return Ok(response),
    };

    Ok(HttpResponse::Ok()
        .append_header((CONTENT_TYPE, "application/x-git-upload-pack-advertisement"))
        .body(capabilities(Some(service)).await?))
}

#[instrument(err, skip(request, db_pool))]
async fn receive_pack_info_refs(repo_option: Option<Repository>, request: &HttpRequest, db_pool: &Pool) -> Result<HttpResponse> {
    let mut tx = db_pool.begin().await?;

    let user = match basic_auth::login_flow(request, &mut tx, "application/x-git-receive-pack-advertisement").await? {
        Either::Left(user) => user,
        Either::Right(response) => return Ok(response),
    };

    let Some(repo) = repo_option else {
        die!(NOT_FOUND);
    };

    if !privilege::check_push(&repo, Some(&user), &mut tx).await? {
        die!(NOT_FOUND);
    }

    let git2repo = repo.libgit2(&mut tx).await?;
    let output = ls_refs_all(&git2repo, Some("git-receive-pack")).await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok()
        .append_header((CONTENT_TYPE, "application/x-git-receive-pack-advertisement"))
        .body(output))
}
