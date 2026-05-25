use chrono::{DateTime, Utc};
use gitarena_issues::operation::BugStatus;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use tracing::error;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::meili::{ISSUES_MEILI_INDEX, MeiliClient};

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
