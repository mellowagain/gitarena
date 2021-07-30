use crate::error::GAErrors::GitError;
use crate::extensions::get_header;
use crate::git::basic_auth;
use crate::git::fetch::fetch;
use crate::git::ls_refs::ls_refs;
use crate::git::reader::read_until_command;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;

use actix_web::{Either, HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use futures::StreamExt;
use git_packetline::{PacketLine, StreamingPeekableIter};
use gitarena_macros::route;
use log::warn;
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

    let mut readable_iter = StreamingPeekableIter::new(vec.as_slice(), &[PacketLine::Flush]);
    readable_iter.fail_on_err_lines(true);

    let mut git_body = Vec::<Vec<u8>>::new();

    /*
    let mut reader = readable_iter.as_read();
    let mut raw_bytes = Vec::<u8>::new();
    reader.read_to_end(&mut raw_bytes).await?;
     */

    while let Some(a) = readable_iter.read_line().await {
        match a {
            Ok(b) => {
                match b {
                    Ok(c) => {
                        match c {
                            PacketLine::Data(d) => {
                                let mut trailing_nl = false;

                                if let Some(last) = d.last() {
                                    if last == &10_u8 { // \n
                                        trailing_nl = true;
                                    }
                                }

                                let length = if trailing_nl {
                                    d.len() - 1
                                } else {
                                    d.len()
                                };

                                git_body.push(d[..length].to_vec());
                            }
                            PacketLine::Flush => {
                                //warn!("flush");
                            }
                            PacketLine::Delimiter => {
                                //warn!("delim");
                            }
                            PacketLine::ResponseEnd => {
                                //warn!("response end");
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse Git body: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to parse Git body: {}", e);
            }
        }
    }

    /*for line_result in raw_bytes.byte_lines() {
        match line_result {
            Ok(line) => {
                git_body.push(line);
            }
            Err(e) => {
                warn!("Failed to parse Git body: {}", e);
            }
        }
    }*/

    transaction.commit().await?;

    let (command, body) = read_until_command(git_body).await?;

    Ok(match command.as_str() {
        "ls-refs" => {
            let output = ls_refs(body, &git2repo).await?;

            HttpResponse::Ok()
                .header("Cache-Control", "no-cache, max-age=0, must-revalidate")
                .header("Content-Type", accept_header)
                .body(output)
        }
        "fetch" => {
            let output = fetch(body, &git2repo).await?;

            HttpResponse::Ok()
                .header("Cache-Control", "no-cache, max-age=0, must-revalidate")
                .header("Content-Type", accept_header)
                .body(output)
        }
        _ => HttpResponse::Unauthorized() // According to spec we have to send unauthorized for commands we don't understand
                .header("Cache-Control", "no-cache, max-age=0, must-revalidate")
                .header("Content-Type", accept_header)
                .finish()
    })
}
