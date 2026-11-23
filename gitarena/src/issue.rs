use std::collections::HashSet;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use fang::{AsyncQueueable, AsyncRunnable};
use gitarena_issues::bug::BUGS_REF_PREFIX;
use gitarena_issues::operation::BugStatus;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Transaction, Type};
use tracing::{debug, error, info};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::config::{get_setting, set_setting};
use crate::database::{Database, Pool};
use crate::issue::task::WriteEmptyBlobTask;
use crate::meili::{ISSUES_MEILI_INDEX, MeiliClient};
use crate::queue::GLOBAL_QUEUE;
use crate::repository::Repository;
use crate::user::User;

pub(crate) mod task;

const ISSUES_GITARENA_VERSION: i32 = 1;

pub(crate) async fn init(db_pool: &Pool) -> Result<()> {
    let mut tx = db_pool.begin().await?;
    let version: i32 = get_setting("issues.gitarena_version", &mut tx).await?;

    if version >= ISSUES_GITARENA_VERSION {
        tx.commit().await?;
        return Ok(());
    }

    info!(
        found = version,
        expected = ISSUES_GITARENA_VERSION,
        "issues gitarena version out of date, writing empty blob to all repos with issues..."
    );

    let repo_ids: Vec<Uuid> = sqlx::query_scalar("select distinct repo_id from issue_cache").fetch_all(&mut *tx).await?;

    for repo_id in repo_ids {
        let task = WriteEmptyBlobTask { repo_id };

        let queued_task = GLOBAL_QUEUE
            .get()
            .ok_or_else(|| anyhow!("empty blob write should only be scheduled after queue has been initialized"))?
            .insert_task(&task as &dyn AsyncRunnable)
            .await?;

        debug!(task.id = %queued_task.id, %repo_id, "queued repo for empty blob write");
    }

    set_setting("issues.gitarena_version", ISSUES_GITARENA_VERSION, &mut tx).await?;

    tx.commit().await?;
    Ok(())
}

#[derive(FromRow, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IssueCache {
    pub(crate) id: Uuid,
    pub(crate) repo_id: Uuid,
    pub(crate) git_bug_id: String,
    pub(crate) index: i32,

    pub(crate) author_id: Uuid,

    pub(crate) title: String,
    pub(crate) body: String,

    pub(crate) labels: Vec<String>,
    pub(crate) priority: String,

    pub(crate) status: IssueStatus,
    pub(crate) confidential: bool,
    pub(crate) locked: bool,

    pub(crate) milestone_id: Option<Uuid>,
    pub(crate) assignees: Vec<Uuid>,

    pub(crate) updated_at: DateTime<Utc>,
}

impl IssueCache {
    pub(crate) async fn index_meili(&self, client: &MeiliClient) {
        if let Some(client) = client
            && let Err(err) = client.index(ISSUES_MEILI_INDEX).add_documents(&[self], Some("id")).await
        {
            error!(?err, "failed to index issue in meilisearch");
        }
    }

    pub(crate) async fn deindex_meili(id: Uuid, client: &MeiliClient) {
        if let Some(client) = client
            && let Err(err) = client.index(ISSUES_MEILI_INDEX).delete_document(id).await
        {
            error!(?err, "failed to deindex issue in meilisearch");
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Type, ToSchema)]
#[sqlx(type_name = "issue_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub(crate) enum IssueStatus {
    Open,
    InProgress,
    Completed,
    NotPlanned,
}

impl IssueStatus {
    pub(crate) fn to_git_bug_status(&self) -> BugStatus {
        match self {
            Self::Open | Self::InProgress => BugStatus::OPEN,
            Self::Completed | Self::NotPlanned => BugStatus::CLOSED,
        }
    }
}

#[derive(FromRow, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IssueCommentCache {
    pub(crate) id: Uuid,
    pub(crate) op_id: String,
    pub(crate) issue_id: Uuid,
    pub(crate) author_id: Uuid,
    pub(crate) body: String,
    pub(crate) edited_at: Option<DateTime<Utc>>,
}

pub(crate) async fn hidden_bug_refs(repo: &Repository, user: Option<&User>, tx: &mut Transaction<'_, Database>) -> Result<HashSet<String>> {
    if user.is_some_and(|user| repo.owner_user == Some(user.id)) {
        return Ok(HashSet::new());
    }

    let bug_ids: Vec<String> = sqlx::query_scalar("select git_bug_id from issue_cache where repo_id = $1 and confidential = true")
        .bind(repo.id)
        .fetch_all(&mut **tx)
        .await?;

    Ok(bug_ids.into_iter().map(|id| format!("{BUGS_REF_PREFIX}{id}")).collect())
}
