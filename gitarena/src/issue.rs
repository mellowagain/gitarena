use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Contains issues and their corresponding data; Does *not* contain the actual text content
#[derive(FromRow, Display, Debug, Serialize)]
#[display("{title}")]
pub(crate) struct Issue {
    pub(crate) id: Uuid,

    repo: Uuid,
    index: i32, // Issue # per repository (not global instance)

    pub(crate) author: Uuid,
    title: String,

    pub(crate) milestone: Option<i32>,
    pub(crate) labels: Vec<i32>,
    pub(crate) assignees: Vec<i32>,

    closed: bool,
    confidential: bool,
    locked: bool,

    #[serde(with = "ts_seconds")]
    updated_at: DateTime<Utc>,
}
