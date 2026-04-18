use crate::user::User;
use chrono::{DateTime, Utc};
use derive_more::Display;
use gitarena_common::database::Database;
use gitarena_common::database::models::KeyType;
use serde::Serialize;
use sqlx::{FromRow, Transaction};

#[derive(FromRow, Display, Debug, Serialize)]
#[display(fmt = "{title}")]
pub(crate) struct SshKey {
    pub(crate) id: i32,
    pub(crate) owner: i32,
    pub(crate) title: String,
    pub(crate) fingerprint: String,
    pub(crate) algorithm: KeyType,
    key: Vec<u8>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) expires_at: Option<DateTime<Utc>>,
}

impl SshKey {
    pub(crate) async fn all_from_user(user: &User, tx: &mut Transaction<'_, Database>) -> Option<Vec<SshKey>> {
        sqlx::query_as::<_, SshKey>("select * from ssh_keys where owner = $1")
            .bind(user.id)
            .fetch_all(&mut **tx)
            .await
            .ok()
    }

    pub(crate) fn as_string(&self) -> String {
        format!("{} {}", &self.algorithm, base64::encode(&self.key))
    }
}
