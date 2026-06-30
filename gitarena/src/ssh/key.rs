use crate::database::Database;
use crate::user::User;
use anyhow::Result;
use anyhow::{Error, bail};
use base64::Engine as _;
use base64::engine::general_purpose;
use chrono::{DateTime, Utc};
use derive_more::Display;
use russh::keys::Algorithm;
use russh::keys::ssh_key::Fingerprint;
use russh::keys::{EcdsaCurve, HashAlg};
use serde::Deserialize;
use serde::{Serialize, Serializer};
use sqlx::Type;
use sqlx::{FromRow, Transaction};
use tracing::instrument;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(FromRow, Display, derive_more::Debug, Serialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
#[display("{title}")]
pub(crate) struct SshKey {
    pub(crate) id: Uuid,
    pub(crate) owner: Uuid,
    pub(crate) title: String,
    pub(crate) fingerprint: String,
    pub(crate) algorithm: KeyType,
    #[serde(rename = "pubkey", serialize_with = "serialize_key_as_base64")]
    #[schema(value_type = String, rename = "pubkey")]
    #[debug(skip)]
    key: Vec<u8>,
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
        format!("{} {}", &self.algorithm, general_purpose::STANDARD.encode(&self.key))
    }
}

fn serialize_key_as_base64<S: Serializer>(key: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&general_purpose::STANDARD.encode(key))
}

#[derive(Type, Debug, Display, Deserialize, Serialize, Copy, Clone, ToSchema)]
#[sqlx(type_name = "ssh_key_type", rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
#[display(rename_all = "kebab-case")]
pub enum KeyType {
    SshRsa,
    #[display("ecdsa-sha2-nistp256")]
    EcdsaSha2Nistp256,
    #[display("ecdsa-sha2-nistp384")]
    EcdsaSha2Nistp384,
    #[display("ecdsa-sha2-nistp521")]
    EcdsaSha2Nistp521,
    #[display("ssh-ed25519")]
    SshEd25519,
    #[sqlx(rename = "sk-ssh-ed25519@openssh.com")]
    #[serde(rename = "sk-ssh-ed25519@openssh.com")]
    #[display("sk-ssh-ed25519@openssh.com")]
    SkSshEd25519,
    #[sqlx(rename = "sk-ecdsa-sha2-nistp256@openssh.com")]
    #[serde(rename = "sk-ecdsa-sha2-nistp256@openssh.com")]
    #[display("sk-ecdsa-sha2-nistp256@openssh.com")]
    SkEcdsaSha2Nistp256,
    #[sqlx(rename = "rsa-sha2-256")]
    #[display("rsa-sha2-256")]
    RsaSha2_256,
    #[sqlx(rename = "rsa-sha2-512")]
    #[display("rsa-sha2-512")]
    RsaSha2_512,
}

impl TryFrom<&str> for KeyType {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "ssh-rsa" => KeyType::SshRsa,
            "ecdsa-sha2-nistp256" => KeyType::EcdsaSha2Nistp256,
            "ecdsa-sha2-nistp384" => KeyType::EcdsaSha2Nistp384,
            "ecdsa-sha2-nistp521" => KeyType::EcdsaSha2Nistp521,
            "ssh-ed25519" => KeyType::SshEd25519,
            "sk-ssh-ed25519@openssh.com" => KeyType::SkSshEd25519,
            "sk-ecdsa-sha2-nistp256@openssh.com" => KeyType::SkEcdsaSha2Nistp256,
            "rsa-sha2-256" => KeyType::RsaSha2_256,
            "rsa-sha2-512" => KeyType::RsaSha2_512,
            _ => bail!("Unknown algorithm: {value}"),
        })
    }
}

impl TryFrom<&Algorithm> for KeyType {
    type Error = Error;

    fn try_from(value: &Algorithm) -> Result<Self, Self::Error> {
        Ok(match value {
            Algorithm::Dsa => bail!("DSA keys are unsupported"),
            Algorithm::Ecdsa { curve } => match curve {
                EcdsaCurve::NistP256 => KeyType::EcdsaSha2Nistp256,
                EcdsaCurve::NistP384 => KeyType::EcdsaSha2Nistp384,
                EcdsaCurve::NistP521 => KeyType::EcdsaSha2Nistp521,
            },
            Algorithm::Rsa { hash: Some(HashAlg::Sha256) } => KeyType::RsaSha2_256,
            Algorithm::Rsa { hash: Some(HashAlg::Sha512) } => KeyType::RsaSha2_512,
            Algorithm::Rsa { hash: None } | Algorithm::Rsa { .. } => KeyType::SshRsa,
            Algorithm::SkEcdsaSha2NistP256 => KeyType::SkEcdsaSha2Nistp256,
            Algorithm::Ed25519 => KeyType::SshEd25519,
            Algorithm::SkEd25519 => KeyType::SkSshEd25519,
            Algorithm::Other(name) => bail!("Unknown algorithm: {}", name.as_str()),
            name => unimplemented!("russh supports {name} but gitarena doesn't?!"),
        })
    }
}
