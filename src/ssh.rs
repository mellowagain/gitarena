use chrono::{DateTime, Utc};
use derive_more::Display;
use gitarena_common::database::models::KeyType;
use serde::Serialize;
use sqlx::FromRow;

#[derive(FromRow, Display, Debug, Serialize)]
#[display(fmt = "{}", title)]
pub(crate) struct SshKey {
    pub(crate) id: i32,
    pub(crate) owner: i32,
    pub(crate) title: String,
    pub(crate) fingerprint: String,
    pub(crate) algorithm: KeyType,
    key: Vec<u8>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) expires_at: Option<DateTime<Utc>>
}
