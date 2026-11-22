use crate::database::Pool;
use crate::die;
use crate::git::GitProtocol;
use crate::git::basic_auth;
use crate::git::upload_pack::{UploadPackV1, execute_upload_pack_v2};
use crate::issue::hidden_bug_refs;
use crate::metrics::git::{OPERATION_COUNT, OPERATION_DURATION};
use crate::prelude::*;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;
use crate::routes::repository::git::info_refs::resolve_namespace;

use std::time::Instant;

use actix_web::dev::Decompress;
use actix_web::http::header::CONTENT_TYPE;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use either::Either;
use futures::StreamExt;
use gitarena_macros::route;
use opentelemetry::KeyValue;

#[route("/{namespace}/{repository}.git/git-upload-pack", method = "POST", err = "git")]
pub(crate) async fn git_upload_pack(uri: web::Path<GitRequest>, body: web::Payload, request: HttpRequest, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let content_type = request.get_header("content-type").unwrap_or_default();
    let accept_header = request.get_header("accept").unwrap_or_default();

    if content_type != "application/x-git-upload-pack-request" || accept_header != "application/x-git-upload-pack-result" {
        die!(BAD_REQUEST);
    }

    let version = GitProtocol::from_parameters(request.get_header("git-protocol").unwrap_or_default());

    let mut transaction = db_pool.begin().await?;

    let owner_id = resolve_namespace(&uri.namespace, &mut transaction).await?;
    let repo_option = Repository::open(owner_id, &uri.repository, &mut transaction).await;

    let (user, repo) = match basic_auth::validate_repo_access(repo_option, "application/x-git-upload-pack-advertisement", &request, &mut transaction).await? {
        Either::Left(tuple) => tuple,
        Either::Right(response) => return Ok(response),
    };

    if !privilege::check_access(&repo, user.as_ref(), &mut transaction).await? {
        die!(NOT_FOUND);
    }

    let hidden_refs = hidden_bug_refs(&repo, user.as_ref(), &mut transaction).await?;
    let git2repo = repo.libgit2(&mut transaction).await?;

    // git gzips request bodies larger than 1kb
    let mut body = Decompress::from_headers(body.into_inner(), request.headers());
    let mut bytes = web::BytesMut::new();

    while let Some(item) = body.next().await {
        let item = item?;
        bytes.extend_from_slice(&item);
    }

    let start = Instant::now();

    let output = match version {
        GitProtocol::V2 => execute_upload_pack_v2(bytes.as_ref(), &git2repo, &hidden_refs).await?,
        GitProtocol::V0 | GitProtocol::V1 => UploadPackV1::stateless().process(bytes.as_ref(), &git2repo, &hidden_refs).await?,
    };

    let elapsed = start.elapsed().as_secs_f64();

    OPERATION_COUNT.add(
        1,
        &[
            KeyValue::new("operation", "upload-pack"),
            KeyValue::new("transport", "http"),
            KeyValue::new("status", "ok"),
        ],
    );

    OPERATION_DURATION.record(elapsed, &[KeyValue::new("operation", "upload-pack"), KeyValue::new("transport", "http")]);

    transaction.commit().await?;

    Ok(HttpResponse::Ok().append_header((CONTENT_TYPE, accept_header)).body(output))
}
