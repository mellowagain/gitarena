use crate::error::GAErrors::GitError;
use crate::extensions::get_header;
use crate::git::basic_auth;
use crate::git::ls_refs::ls_refs;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;

use actix_web::{Either, HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use bstr::io::BufReadExt;
use futures::StreamExt;
use git_packetline::{PacketLine, Provider};
use gitarena_macros::route;
use sqlx::PgPool;

#[route("/{username}/{repository}.git/git-upload-pack", method="POST")]
pub(crate) async fn git_upload_pack(uri: web::Path<GitRequest>, mut body: web::Payload, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let content_type = get_header(&request, "Content-Type").unwrap_or_default();
    let accept_header = get_header(&request, "Accept").unwrap_or_default();

    if content_type != "application/x-git-upload-pack-request" || accept_header != "application/x-git-upload-pack-result" {
        return Err(GitError(400, None).into());
    }

    let git_protocol = get_header(&request, "Git-Protocol").unwrap_or_default();

    if git_protocol != "version=2" {
        return Err(GitError(400, Some("Unsupported Git protocol version".to_owned())).into());
    }

    let mut transaction = db_pool.begin().await?;

    let user_option: Option<(i32,)> = sqlx::query_as("select id from users where lower(username) = lower($1)")
        .bind(&uri.username)
        .fetch_optional(&mut transaction)
        .await?;

    let (user_id,) = match user_option {
        Some(user_id) => user_id,
        None => return Err(GitError(404, None).into())
    };

    let repo_option: Option<Repository> = sqlx::query_as::<_, Repository>("select * from repositories where owner = $1 and lower(name) = lower($2)")
        .bind(user_id)
        .bind(&uri.repository)
        .fetch_optional(&mut transaction)
        .await?;

    let (_, repo) = match basic_auth::validate_repo_access(repo_option, "application/x-git-upload-pack-advertisement", &request, &mut transaction).await? {
        Either::A(tuple) => tuple,
        Either::B(response) => return Ok(response)
    };

    let git2repo = repo.libgit2(&uri.username).await?;

    let mut bytes = web::BytesMut::new();

    while let Some(item) = body.next().await {
        let item = item?;
        bytes.extend_from_slice(&item);
    }

    let frozen_bytes = bytes.freeze();
    let vec = frozen_bytes.to_vec();

    let mut provider = Provider::new(vec.as_slice(), &[PacketLine::Flush]);
    let mut git_body = Vec::<Vec<u8>>::new();

    /*let mut callback = |is_err: bool, data: &[u8]| {
        git_body.push(data);
    };

    provider.as_read_with_sidebands(&mut callback);*/
    let reader = provider.as_read();

    for line_result in reader.byte_lines() {
        match line_result {
            Ok(line) => {
                git_body.push(line);
            }
            Err(_) => { /* ignore */}
        }
    }

    if let Some(first) = git_body.first() {
        match String::from_utf8(first.to_vec()) {
            Ok(first_line) => {
                if first_line.starts_with("command") {
                    let mut splitted = first_line.splitn(2, "=");
                    let _ = splitted.next().unwrap_or_default();
                    let command = splitted.next().unwrap_or_default();

                    if !command.is_empty() {
                        if command == "ls-refs" {
                            git_body.remove(0);
                            let output = ls_refs(git_body, &git2repo).await?;

                            return Ok(HttpResponse::Ok()
                                .header("Cache-Control", "no-cache, max-age=0, must-revalidate")
                                .header("Content-Type", accept_header)
                                .body(output));
                        }
                    }
                }
            }
            Err(_) => { /* ignore */}
        }
    }

    transaction.commit().await?;

    Ok(HttpResponse::InternalServerError().finish())
}
