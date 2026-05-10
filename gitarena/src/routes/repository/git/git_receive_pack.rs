use crate::die;
use crate::git::basic_auth;
use crate::git::receive_pack::execute_receive_pack;
use crate::metrics::git::{OPERATION_COUNT, OPERATION_DURATION};
use crate::prelude::*;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;
use gitarena_common::database::Pool;

use std::time::Instant;

use actix_web::http::header::CONTENT_TYPE;
use actix_web::{Either, HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use futures::StreamExt;
use gitarena_macros::route;
use opentelemetry::KeyValue;
use uuid::Uuid;

#[route("/{username}/{repository}.git/git-receive-pack", method = "POST", err = "git")]
pub(crate) async fn git_receive_pack(
    uri: web::Path<GitRequest>,
    mut body: web::Payload,
    request: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let start = Instant::now();
    let content_type = request.get_header("content-type").unwrap_or_default();
    let accept_header = request.get_header("accept").unwrap_or_default();

    if content_type != "application/x-git-receive-pack-request" || accept_header != "application/x-git-receive-pack-result" {
        die!(BAD_REQUEST);
    }

    let mut transaction = db_pool.begin().await?;

    let user_option: Option<(Uuid,)> = sqlx::query_as("select id from users where lower(username) = lower($1) limit 1")
        .bind(&uri.username)
        .fetch_optional(&mut *transaction)
        .await?;

    let Some((user_id,)) = user_option else { die!(NOT_FOUND) };

    let repo_option: Option<Repository> = sqlx::query_as::<_, Repository>("select * from repositories where owner = $1 and lower(name) = lower($2) limit 1")
        .bind(user_id)
        .bind(&uri.repository)
        .fetch_optional(&mut *transaction)
        .await?;

    let user = match basic_auth::login_flow(&request, &mut transaction, "application/x-git-receive-pack-result").await? {
        Either::Left(user) => user,
        Either::Right(response) => return Ok(response),
    };

    let Some(mut repo) = repo_option else { die!(NOT_FOUND) };

    // If the user doesn't have access return 404 Not found to not leak existence of internal/private repositories
    if !privilege::check_access(&repo, Some(&user), &mut transaction).await? {
        die!(NOT_FOUND)
    }

    if !privilege::check_push(&repo, Some(&user), &mut transaction).await? {
        die!(UNAUTHORIZED, "No permission to push into this repo");
    }

    if repo.archived {
        die!(UNAUTHORIZED, "Repository is archived and thus read-only");
    }

    transaction.commit().await?;

    let mut bytes = web::BytesMut::new();

    while let Some(item) = body.next().await {
        let item = item?;
        bytes.extend_from_slice(&item);
    }

    let data = bytes.freeze();
    let output_writer = execute_receive_pack(&db_pool, &mut repo, &data).await?;
    let output = output_writer.serialize().await?;

    if output.is_empty() {
        return Ok(HttpResponse::NoContent().append_header((CONTENT_TYPE, accept_header)).finish());
    }

    let elapsed = start.elapsed().as_secs_f64();

    OPERATION_COUNT.add(
        1,
        &[
            KeyValue::new("operation", "push"),
            KeyValue::new("transport", "http"),
            KeyValue::new("status", "ok"),
        ],
    );

    OPERATION_DURATION.record(elapsed, &[KeyValue::new("operation", "push"), KeyValue::new("transport", "http")]);

    Ok(HttpResponse::Ok().append_header((CONTENT_TYPE, accept_header)).body(output))
}
