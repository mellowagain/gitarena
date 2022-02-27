use crate::die;
use crate::git::hooks::post_update;
use crate::git::io::band::Band;
use crate::git::io::reader::read_data_lines;
use crate::git::io::writer::GitWriter;
use crate::git::receive_pack::{process_create_update, process_delete};
use crate::git::ref_update::{RefUpdate, RefUpdateType};
use crate::git::{basic_auth, pack, ref_update};
use crate::prelude::*;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;

use std::path::Path;

use actix_web::http::header::CONTENT_TYPE;
use actix_web::{Either, HttpRequest, HttpResponse, Responder, web};
use anyhow::{Context, Result};
use async_process::{Command, Stdio};
use futures::StreamExt;
use git_repository::protocol::transport::packetline::{PacketLineRef, StreamingPeekableIter};
use gitarena_macros::route;
use log::warn;
use memmem::{Searcher, TwoWaySearcher};
use sqlx::PgPool;

#[route("/{username}/{repository}.git/git-receive-pack", method = "POST", err = "git")]
pub(crate) async fn git_receive_pack(uri: web::Path<GitRequest>, mut body: web::Payload, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let content_type = request.get_header("content-type").unwrap_or_default();
    let accept_header = request.get_header("accept").unwrap_or_default();

    if content_type != "application/x-git-receive-pack-request" || accept_header != "application/x-git-receive-pack-result" {
        die!(BAD_REQUEST);
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

    let user = match basic_auth::login_flow(&request, &mut transaction, "application/x-git-receive-pack-result").await? {
        Either::Left(user) => user,
        Either::Right(response) => return Ok(response)
    };

    let mut repo = match repo_option {
        Some(repo) => repo,
        None => die!(NOT_FOUND)
    };

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

    let mut bytes = web::BytesMut::new();

    while let Some(item) = body.next().await {
        let item = item?;
        bytes.extend_from_slice(&item);
    }

    let frozen_bytes = bytes.freeze();
    let vec = &frozen_bytes[..];

    let mut readable_iter = StreamingPeekableIter::new(vec, &[PacketLineRef::Flush]);
    readable_iter.fail_on_err_lines(true);

    let git_body = read_data_lines(&mut readable_iter).await?;
    let mut updates = Vec::<RefUpdate>::new();

    for line in git_body {
        updates.push(ref_update::parse_line(line).await?);
    }

    if updates.is_empty() {
        warn!("Upload pack ref update list provided by client is empty");

        return Ok(HttpResponse::NoContent()
            .append_header((CONTENT_TYPE, accept_header))
            .finish());
    }

    let gitoxide_repo = repo.gitoxide(&mut transaction).await?;
    let store = gitoxide_repo.objects.clone();

    let mut output_writer = GitWriter::new();

    let searcher = TwoWaySearcher::new(b"PACK");

    match searcher.search_in(vec) {
        Some(pos) => {
            let (index_path, pack_path, _temp_dir) = pack::read(&vec[pos..], &repo, &mut transaction).await?;

            output_writer.write_text_sideband_pktline(Band::Data, "unpack ok").await?;

            for update in updates {
                match RefUpdateType::determinate(&update.old, &update.new).await? {
                    RefUpdateType::Create | RefUpdateType::Update => process_create_update(&update, &repo, store.clone(), &db_pool, &mut output_writer, index_path.as_ref(), pack_path.as_ref(), &vec[pos..]).await?,
                    RefUpdateType::Delete => process_delete(&update, &repo, &mut transaction, &mut output_writer).await?
                };
            }
        }
        None => {
            if !ref_update::is_only_deletions(updates.as_slice()).await? {
                warn!("Client sent no PACK file despite having more than just deletions");
                die!(BAD_REQUEST, "No PACK payload was sent");
            }

            // There wasn't actually something to unpack
            output_writer.write_text_sideband_pktline(Band::Data, "unpack ok").await?;

            for update in updates {
                process_delete(&update, &repo, &mut transaction, &mut output_writer).await?;
            }
        }
    }

    let repo_dir_str = repo.get_fs_path(&mut transaction).await?;
    let repo_dir = Path::new(&repo_dir_str);

    // Let Git collect garbage to optimize repo size
    match Command::new("git").args(&["gc", "--auto", "--quiet"]).current_dir(repo_dir).stdout(Stdio::null()).stderr(Stdio::null()).status().await {
        Ok(status) => if !status.success() {
            warn!("Git garbage collector exited with non-zero status: {}", status);
        }
        Err(err) => warn!("Failed to execute Git garbage collector: {}", err)
    }

    output_writer.flush_sideband(Band::Data).await?;
    output_writer.flush().await?;

    // Run post update hooks
    post_update::run(store, &mut repo, &mut transaction)
        .await
        .with_context(|| format!("Failed to run post update hook for newest commit in {}/{}", &uri.username, repo.name))?;

    sqlx::query("update repositories set license = $1 where id = $2")
        .bind(&repo.license)
        .bind(&repo.id)
        .execute(&mut transaction)
        .await?;

    transaction.commit().await?;

    Ok(HttpResponse::Ok()
        .append_header((CONTENT_TYPE, accept_header))
        .body(output_writer.serialize().await?))
}
