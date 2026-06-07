use crate::config::{get_setting, set_setting};
use crate::contributions::task::BackfillRepoContributionsTask;
use crate::database::{Database, Pool};
use crate::git::ref_update::RefUpdate;
use crate::queue::GLOBAL_QUEUE;
use crate::repository::Repository;
use crate::utils::oid;
use anyhow::{Result, anyhow};
use chrono::{NaiveDate, TimeZone, Utc};
use fang::{AsyncQueueable, AsyncRunnable};
use gix::hash::ObjectId;
use gix::objs::{CommitRef, Kind};
use gix::odb::Store;
use gix::odb::pack::FindExt;
use sqlx::Transaction;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tracing::{debug, info, instrument};
use uuid::Uuid;

pub(crate) mod task;

const CONTRIBUTIONS_GITARENA_VERSION: i32 = 1;

/// (sha_hex, email_lower, unix_seconds)
type WalkCommit = (String, String, i64);

pub(crate) async fn init(db_pool: &Pool) -> Result<()> {
    let mut tx = db_pool.begin().await?;
    let version: i32 = get_setting("contributions.gitarena_version", &mut tx).await?;

    if version >= CONTRIBUTIONS_GITARENA_VERSION {
        return Ok(());
    }

    info!(
        found = version,
        expected = CONTRIBUTIONS_GITARENA_VERSION,
        "contributions gitarena version out of date, backfilling all repos..."
    );

    let repos: Vec<Repository> = sqlx::query_as("select * from repositories").fetch_all(&mut *tx).await?;

    for repo in repos {
        let task = BackfillRepoContributionsTask { repo_id: repo.id };

        let queued_task = GLOBAL_QUEUE
            .get()
            .ok_or_else(|| anyhow!("contributions backfill should only be scheduled after queue has been initialized"))?
            .insert_task(&task as &dyn AsyncRunnable)
            .await?;

        debug!(task.id = %queued_task.id, %repo.id, "queued repo for contribution backfill");
    }

    set_setting("contributions.gitarena_version", CONTRIBUTIONS_GITARENA_VERSION, &mut tx).await?;

    tx.commit().await?;
    Ok(())
}

#[instrument(skip(store))]
pub(crate) fn walk_new_commits(store: &Arc<Store>, old_oid: Option<ObjectId>, new_oid: ObjectId) -> (bool, Vec<WalkCommit>) {
    let cache = store.to_cache_arc();

    let mut buffer = Vec::<u8>::new();
    let mut visited: HashSet<ObjectId> = HashSet::new();
    let mut commits: Vec<WalkCommit> = Vec::new();

    let mut queue: VecDeque<ObjectId> = VecDeque::new();
    queue.push_back(new_oid);

    while let Some(oid) = queue.pop_front() {
        if old_oid.is_some_and(|o| oid == o) {
            return (true, commits);
        }

        if !visited.insert(oid) {
            continue;
        }

        if visited.len() > 10_000 {
            break;
        }

        let (email_lower, timestamp, parents) = {
            let Ok((data, _)) = cache.find(oid.as_ref(), &mut buffer) else { continue };

            if data.kind != Kind::Commit {
                continue;
            }

            let Ok(commit) = CommitRef::from_bytes(data.data) else { continue };
            let Ok(email) = std::str::from_utf8(commit.author.email) else { continue };
            let parents: Vec<ObjectId> = commit.parents.iter().filter_map(|p| ObjectId::from_hex(p).ok()).collect();

            (email.to_lowercase(), commit.author.seconds(), parents)
        };

        commits.push((format!("{oid}"), email_lower, timestamp));

        for parent in parents {
            queue.push_back(parent);
        }
    }

    (false, commits)
}

#[instrument(err, skip(updates, store, tx))]
pub(crate) async fn record_commit_contributions(updates: &[RefUpdate], store: &Arc<Store>, repo_id: Uuid, tx: &mut Transaction<'_, Database>) -> Result<()> {
    let mut all_commits: Vec<(String, String, NaiveDate)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for update in updates {
        if !update.target_ref.starts_with("refs/heads/") {
            continue;
        }

        let Some(new_hex) = &update.new else {
            continue;
        };

        let new_oid = oid::from_hex_str(Some(new_hex))?;
        let old_oid = update.old.as_deref().map(|hash| oid::from_hex_str(Some(hash))).transpose()?;

        let (_, commits) = walk_new_commits(store, old_oid, new_oid);

        for (sha, email, timestamp) in commits {
            if !seen.insert(sha.clone()) {
                continue;
            }

            let Some(dt) = Utc.timestamp_opt(timestamp, 0).single() else { continue };

            all_commits.push((sha, email, dt.date_naive()));
        }
    }

    insert_contributions(repo_id, &all_commits, tx).await
}

#[instrument(err, skip(commit_data, tx))]
pub(crate) async fn insert_contributions(repo_id: Uuid, commit_data: &[(String, String, NaiveDate)], tx: &mut Transaction<'_, Database>) -> Result<()> {
    if commit_data.is_empty() {
        return Ok(());
    }

    let emails: Vec<String> = commit_data.iter().map(|(_, e, _)| e.clone()).collect::<HashSet<_>>().into_iter().collect();

    let email_rows: Vec<(String, Uuid)> = sqlx::query_as("select lower(email), owner from emails where lower(email) = any($1)")
        .bind(&emails)
        .fetch_all(&mut **tx)
        .await?;

    let email_to_user: HashMap<String, Uuid> = email_rows.into_iter().collect();

    let mut user_ids: Vec<Uuid> = Vec::new();
    let mut shas: Vec<String> = Vec::new();
    let mut dates: Vec<NaiveDate> = Vec::new();

    for (sha, email, date) in commit_data {
        if let Some(&user_id) = email_to_user.get(email) {
            user_ids.push(user_id);
            shas.push(sha.clone());
            dates.push(*date);
        }
    }

    if user_ids.is_empty() {
        return Ok(());
    }

    sqlx::query(
        "insert into commit_contributions (user_id, repo_id, commit_sha, author_date) \
         select unnest($1::uuid[]), $2, unnest($3::text[]), unnest($4::date[]) \
         on conflict do nothing",
    )
    .bind(&user_ids)
    .bind(repo_id)
    .bind(&shas)
    .bind(&dates)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
