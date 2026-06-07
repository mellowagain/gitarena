use crate::events::Event;
use crate::git::hooks::post_update;
use crate::git::io::band::Band;
use crate::git::io::reader::read_data_lines;
use crate::git::io::writer::GitWriter;
use crate::git::ref_update::{RefUpdate, RefUpdateType};
use crate::git::{GIT_CLI_AVAILABLE, ref_update};
use crate::prelude::*;
use crate::repository::Repository;
use crate::utils::oid;
use crate::{die, err};

use std::collections::{HashSet, VecDeque};
use std::convert::TryInto;
use std::io::Write;
use std::ops::Deref;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use crate::database::Database;
use crate::database::Pool;
use crate::meili::MeiliClient;
use actix_web::HttpRequest;
use anyhow::{Context, Result, anyhow};
use bstr::BString;
use gix::actor::Signature;
use gix::date::parse::TimeBuf;
use gix::hash::ObjectId;
use gix::lock::acquire::Fail;
use gix::objs::{CommitRef, Kind, TagRef};
use gix::odb::Store;
use gix::odb::pack::FindExt;
use gix::protocol::transport::packetline::PacketLineRef;
use gix::protocol::transport::packetline::async_io::StreamingPeekableIter;
use gix::refs::Target;
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
use memmem::{Searcher, TwoWaySearcher};
use serde_json::{Value, json};
use sqlx::Transaction;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{instrument, warn};
use uuid::Uuid;

#[instrument(err, skip(writer, store))]
pub(crate) async fn process_create_update(ref_update: &RefUpdate, repo: &Repository, store: Arc<Store>, db_pool: &Pool, writer: &mut GitWriter) -> Result<()> {
    assert!(ref_update.new.is_some());

    let mut transaction = db_pool.begin().await?;
    let new_oid = oid::from_hex_str(ref_update.new.as_deref())?;

    // # Gitoxide zone
    // The pack file was written by libgit2 before so we can now just search for the entries
    // and then create a Gitoxide commit and write it to the reflog using a transaction
    {
        let mut buffer = Vec::<u8>::new();

        let (committer, message) = {
            let (data, _) = store
                .to_cache_arc()
                .find(new_oid.as_ref(), &mut buffer)
                .map_err(|_| anyhow!("Failed to find object {new_oid} in ODB after pack write"))?;

            match data.kind {
                Kind::Commit => {
                    let commit = CommitRef::from_bytes(data.data)?;
                    (commit.committer.to_owned()?, BString::from(commit.message))
                }
                Kind::Tag => {
                    let tag = TagRef::from_bytes(data.data)?;
                    let tagger = tag.tagger.ok_or_else(|| anyhow!("Pushed tag object has no tagger field"))?;
                    (tagger.to_owned()?, BString::from(tag.message))
                }
                _ => die!(BAD_REQUEST, "Unexpected payload data type"),
            }
        };

        let previous_value = if let Some(previous_oid_str) = &ref_update.old {
            let previous_oid = oid::from_hex_str(Some(previous_oid_str.as_str()))?;
            let previous_target = Target::Object(previous_oid);

            PreviousValue::ExistingMustMatch(previous_target)
        } else {
            PreviousValue::Any
        };

        let edits = vec![RefEdit {
            change: Change::Update {
                log: LogChange {
                    mode: RefLog::AndReference,
                    force_create_reflog: true,
                    message,
                },
                expected: previous_value,
                new: Target::Object(new_oid),
            },
            name: ref_update.target_ref.as_str().try_into()?,
            deref: true,
        }];

        let gitoxide_repo = repo.gitoxide(&mut transaction).await?;

        gitoxide_repo
            .refs
            .transaction()
            .prepare(edits, Fail::Immediately, Fail::Immediately)
            .map_err(|err| anyhow!("Failed to commit transaction: {err}"))?
            .commit(committer.to_ref(&mut TimeBuf::default()))?;
    }

    if ref_update.report_status || ref_update.report_status_v2 {
        writer.write_text_sideband_pktline(Band::Data, format!("ok {}", ref_update.target_ref)).await?;
    }

    Ok(())
}

#[instrument(err, skip(tx, writer))]
pub(crate) async fn process_delete(ref_update: &RefUpdate, repo: &Repository, tx: &mut Transaction<'_, Database>, writer: &mut GitWriter) -> Result<()> {
    assert!(ref_update.old.is_some());
    assert!(ref_update.new.is_none());

    let gitoxide_repo = repo.gitoxide(tx).await?;

    let object_id = oid::from_hex_str(ref_update.old.as_deref()).map_err(|_| err!(NOT_FOUND, "Ref does not exist"))?;

    let edits = vec![RefEdit {
        change: Change::Delete {
            expected: PreviousValue::MustExistAndMatch(Target::Object(object_id)),
            log: RefLog::AndReference,
        },
        name: ref_update.target_ref.as_str().try_into()?,
        deref: true,
    }];

    gitoxide_repo
        .refs
        .transaction()
        .prepare(edits, Fail::Immediately, Fail::Immediately)
        .map_err(|err| err!(INTERNAL_SERVER_ERROR, "Failed to commit transaction: {}", err))?
        .commit(Signature::gitarena_default().to_ref(&mut TimeBuf::default()))?;

    if ref_update.report_status || ref_update.report_status_v2 {
        writer.write_text_sideband_pktline(Band::Data, format!("ok {}", ref_update.target_ref)).await?;
    }

    Ok(())
}

#[instrument(err, skip(db_pool, data, request))]
pub(crate) async fn execute_receive_pack(
    db_pool: &Pool,
    meili_client: &MeiliClient,
    repo: &mut Repository,
    data: &[u8],
    actor_id: Uuid,
    request: Option<&HttpRequest>,
) -> Result<GitWriter> {
    let mut tx = db_pool.begin().await?;

    let mut readable_iter = StreamingPeekableIter::new(data, &[PacketLineRef::Flush], false);
    readable_iter.fail_on_err_lines(true);

    let git_body = read_data_lines(&mut readable_iter).await?;
    let mut updates = Vec::<RefUpdate>::new();

    for line in git_body {
        updates.push(ref_update::parse_line(line).await?);
    }

    if updates.is_empty() {
        warn!("Receive-pack ref update list provided by client is empty");
        return Ok(GitWriter::new());
    }

    ref_update::propagate_capabilities(&mut updates);

    for update in &updates {
        if update.target_ref.starts_with("refs/bugs/") {
            die!(FORBIDDEN, "Issues are managed through the GitArena web interface and cannot be pushed directly");
        }
    }

    let gitoxide_repo = repo.gitoxide(&mut tx).await?;
    let store = gitoxide_repo.objects.store().clone();

    let mut output_writer = GitWriter::new();
    let searcher = TwoWaySearcher::new(b"PACK");

    if let Some(pos) = searcher.search_in(data) {
        {
            let git2_repo = repo.libgit2(&mut tx).await?;
            let odb = git2_repo.odb()?;
            let mut pack_writer = odb.packwriter()?;
            pack_writer.write_all(&data[pos..])?;
            pack_writer.commit()?;
        }

        output_writer.write_text_sideband_pktline(Band::Data, "unpack ok").await?;

        for update in &updates {
            match RefUpdateType::determinate(&update.old, &update.new)? {
                RefUpdateType::Create | RefUpdateType::Update => process_create_update(update, repo, store.clone(), db_pool, &mut output_writer).await?,
                RefUpdateType::Delete => process_delete(update, repo, &mut tx, &mut output_writer).await?,
            }
        }
    } else {
        if !ref_update::is_only_deletions(updates.as_slice())? {
            die!(BAD_REQUEST, "No PACK payload was sent");
        }

        output_writer.write_text_sideband_pktline(Band::Data, "unpack ok").await?;

        for update in &updates {
            process_delete(update, repo, &mut tx, &mut output_writer).await?;
        }
    }

    for update in &updates {
        let events: Vec<(&'static str, Value)> = match RefUpdateType::determinate(&update.old, &update.new)? {
            RefUpdateType::Create if update.target_ref.starts_with("refs/heads/") => {
                let after = update.new.as_deref().unwrap_or_default();
                let (_, commits) = classify_push(&store, None, after)?;

                vec![
                    ("git.branch_created", json!({ "ref": update.target_ref })),
                    (
                        "git.push",
                        json!({ "ref": update.target_ref, "before": null, "after": after, "commits": commits }),
                    ),
                ]
            }
            RefUpdateType::Create if update.target_ref.starts_with("refs/tags/") => {
                let sha = update.new.as_deref().unwrap_or_default();
                vec![("git.tag_created", json!({ "ref": update.target_ref, "sha": sha }))]
            }
            RefUpdateType::Delete if update.target_ref.starts_with("refs/heads/") => {
                let was = update.old.as_deref().unwrap_or_default();
                vec![("git.branch_deleted", json!({ "ref": update.target_ref, "was": was }))]
            }
            RefUpdateType::Delete if update.target_ref.starts_with("refs/tags/") => vec![("git.tag_deleted", json!({ "ref": update.target_ref }))],
            RefUpdateType::Update if update.target_ref.starts_with("refs/heads/") => {
                let before = update.old.as_deref().unwrap_or_default();
                let after = update.new.as_deref().unwrap_or_default();

                let (force, commits) = classify_push(&store, Some(before), after)?;

                if force {
                    vec![("git.force_push", json!({ "ref": update.target_ref, "before": before, "after": after }))]
                } else {
                    vec![(
                        "git.push",
                        json!({ "ref": update.target_ref, "before": before, "after": after, "commits": commits }),
                    )]
                }
            }
            _ => vec![],
        };

        for (event_type, mut payload) in events {
            let event = match request {
                Some(req) => {
                    payload["transport"] = Value::String("http".to_string());
                    Event::new(event_type, actor_id, req, repo.deref().into(), Some(payload))
                }
                None => {
                    payload["transport"] = Value::String("ssh".to_string());
                    Event::new_without_request(event_type, actor_id, repo.deref().into(), Some(payload))
                }
            };

            event.save(&mut tx).await?;
        }
    }

    let repo_dir_str = repo.get_fs_path(&mut tx).await?;
    let repo_dir = Path::new(&repo_dir_str).to_owned();

    if *GIT_CLI_AVAILABLE {
        let command = Command::new("git")
            .args(["gc", "--auto", "--quiet"])
            .current_dir(&repo_dir)
            .kill_on_drop(true)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match timeout(Duration::from_secs(10), command).await {
            Ok(Ok(status)) => {
                if !status.success() {
                    let exit_code = status.code().map_or_else(|| "unknown".to_string(), |code| code.to_string());
                    warn!(exit_code, "Git garbage collector exited with non-zero status");
                }
            }
            Ok(Err(err)) => warn!(?err, "Failed to execute Git garbage collector"),
            Err(_) => warn!("Git garbage collector failed to finish within 10 seconds"),
        }
    }

    output_writer.flush_sideband(Band::Data).await?;
    output_writer.flush().await?;

    post_update::run(store, repo, &mut tx)
        .await
        .with_context(|| format!("Failed to run post update hook for repo {}", repo.name))?;

    sqlx::query("update repositories set license = $1, languages = $2 where id = $3")
        .bind(&repo.license)
        .bind(&repo.languages)
        .bind(repo.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    repo.index_meili(&meili_client).await;

    Ok(output_writer)
}

/// returns whether the commit was a force push and the amount of commits
#[instrument(err, skip(store))]
fn classify_push(store: &Arc<Store>, old_hex: Option<&str>, new_hex: &str) -> Result<(bool, usize)> {
    let old_oid = old_hex.map(|h| oid::from_hex_str(Some(h))).transpose()?;
    let new_oid = oid::from_hex_str(Some(new_hex))?;

    let cache = store.to_cache_arc();
    let mut buffer = Vec::new();

    let mut visited: HashSet<ObjectId> = HashSet::new();

    let mut queue: VecDeque<ObjectId> = VecDeque::new();
    queue.push_back(new_oid);

    while let Some(oid) = queue.pop_front() {
        if old_oid.is_some_and(|o| oid == o) {
            return Ok((false, visited.len()));
        }

        if !visited.insert(oid) {
            continue;
        }

        if visited.len() > 10_000 {
            break;
        }

        let parents: Vec<ObjectId> = {
            let Ok((data, _)) = cache.find(oid.as_ref(), &mut buffer) else {
                continue;
            };

            if data.kind != Kind::Commit {
                continue;
            }

            let Ok(commit) = CommitRef::from_bytes(data.data) else {
                continue;
            };

            commit.parents.iter().filter_map(|p| ObjectId::from_hex(*p).ok()).collect()
        };

        for parent in parents {
            queue.push_back(parent);
        }
    }

    Ok((old_oid.is_some(), visited.len()))
}
