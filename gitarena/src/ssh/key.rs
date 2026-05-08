use crate::user::User;
use anyhow::Result;
use chrono::{DateTime, Utc};
use derive_more::Display;
use gitarena_common::database::Database;
use gitarena_common::database::models::KeyType;
use russh::keys::Algorithm;
use russh::keys::ssh_key::Fingerprint;
use serde::{Serialize, Serializer};
use sqlx::{FromRow, Transaction};
use tracing::instrument;

#[derive(FromRow, Display, derive_more::Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
#[display("{title}")]
pub(crate) struct SshKey {
    pub(crate) id: i32,
    pub(crate) owner: i32,
    pub(crate) title: String,
    pub(crate) fingerprint: String,
    pub(crate) algorithm: KeyType,
    #[serde(rename = "pubkey", serialize_with = "serialize_key_as_base64")]
    #[debug(skip)]
    key: Vec<u8>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) expires_at: Option<DateTime<Utc>>,
}

impl SshKey {
    #[instrument(skip(tx))]
    pub(crate) async fn all_from_user(user: &User, tx: &mut Transaction<'_, Database>) -> Option<Vec<SshKey>> {
        sqlx::query_as::<_, SshKey>("select * from ssh_keys where owner = $1")
            .bind(user.id)
            .fetch_all(&mut **tx)
            .await
            .ok()
    }

    #[instrument(skip(tx))]
    pub(crate) async fn find(algorithm: &Algorithm, fingerprint: &Fingerprint, tx: &mut Transaction<'_, Database>) -> Result<Option<SshKey>> {
        let algorithm = KeyType::try_from(algorithm)?;

        let fingerprint_str = fingerprint.to_string();
        let fingerprint = fingerprint_str.strip_prefix("SHA256:").unwrap_or(&fingerprint_str);

        Ok(sqlx::query_as::<_, SshKey>("select * from ssh_keys where algorithm = $1 and fingerprint = $2")
            .bind(algorithm)
            .bind(fingerprint)
            .fetch_optional(&mut **tx)
            .await?)
    }

    pub(crate) fn as_string(&self) -> String {
        format!("{} {}", &self.algorithm, base64::encode(&self.key))
    }
}

fn serialize_key_as_base64<S: Serializer>(key: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&base64::encode(key))
}
