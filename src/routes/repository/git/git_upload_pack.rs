use crate::die;
use crate::git::basic_auth;
use crate::git::fetch::fetch;
use crate::git::io::reader::{read_data_lines, read_until_command};
use crate::git::ls_refs::ls_refs;
use crate::prelude::*;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;

use actix_web::{Either, HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use futures::StreamExt;
use git_repository::protocol::transport::packetline::{PacketLineRef, StreamingPeekableIter};
use gitarena_macros::route;
use sqlx::PgPool;

#[route("/{username}/{repository}.git/git-upload-pack", method = "POST", err = "git")]
pub(crate) async fn git_upload_pack(uri: web::Path<GitRequest>, mut body: web::Payload, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let content_type = request.get_header("content-type").unwrap_or_default();
    let accept_header = request.get_header("accept").unwrap_or_default();

    if content_type != "application/x-git-upload-pack-request" || accept_header != "application/x-git-upload-pack-result" {
        die!(BAD_REQUEST);
    }

    let git_protocol = request.get_header("git-protocol").unwrap_or_default();

    if git_protocol != "version=2" {
        die!(BAD_REQUEST, "Unsupported Git protocol version");
    }

    let mut transaction = db_pool.begin().await?;

    let user_option: Option<(i32,)> = sqlx::query_as("select id from users where lower(username) = lower($1) limit 1")
        .bind(&uri.username)
        .fetch_optional(&mut transaction)
        .await?;

    let (user_id,) = match user_option {
        Some(user_id) => user_id,
        None => die!(NOT_FOUND)
    };

    let repo_option: Option<Repository> = sqlx::query_as::<_, Repository>("select * from repositories where owner = $1 and lower(name) = lower($2) limit 1")
        .bind(user_id)
        .bind(&uri.repository)
        .fetch_optional(&mut transaction)
        .await?;

    let (user, repo) = match basic_auth::validate_repo_access(repo_option, "application/x-git-upload-pack-advertisement", &request, &mut transaction).await? {
        Either::A(tuple) => tuple,
        Either::B(response) => return Ok(response)
    };

    if !privilege::check_access(&repo, user.as_ref(), &mut transaction).await? {
        die!(NOT_FOUND);
    }

    let git2repo = repo.libgit2(&mut transaction).await?;

    let mut bytes = web::BytesMut::new();

    while let Some(item) = body.next().await {
        let item = item?;
        bytes.extend_from_slice(&item);
    }

    let frozen_bytes = bytes.freeze();
    let vec = frozen_bytes.to_vec();

    let mut readable_iter = StreamingPeekableIter::new(vec.as_slice(), &[PacketLineRef::Flush]);
    readable_iter.fail_on_err_lines(true);

    let git_body = read_data_lines(&mut readable_iter).await?;
    let (command, body) = read_until_command(git_body).await?;

    let response = match command.as_str() {
        "ls-refs" => {
            let output = ls_refs(body, &git2repo).await?;

            HttpResponse::Ok()
                .header("Content-Type", accept_header)
                .body(output)
        }
        "fetch" => {
            let output = fetch(body, &git2repo).await?;

            HttpResponse::Ok()
                .header("Content-Type", accept_header)
                .body(output)
        }
        _ => HttpResponse::Unauthorized() // According to spec we have to send unauthorized for commands we don't understand
                .header("Content-Type", accept_header)
                .finish()
    };

    transaction.commit().await?;

    Ok(response)
}
