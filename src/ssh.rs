use std::fmt::{Display, Formatter};
use std::fmt;

use anyhow::{bail, Error};
use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::Serialize;
use sqlx::{FromRow, Type};

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

#[derive(Type, Debug, Serialize)]
#[sqlx(type_name = "ssh_key_type", rename_all = "kebab-case")]
pub(crate) enum KeyType {
    SshRsa,
    EcdsaSha2Nistp256,
    EcdsaSha2Nistp384,
    EcdsaSha2Nistp521,
    SshEd25519
}

impl Display for KeyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            KeyType::SshRsa => "ssh-rsa",
            KeyType::EcdsaSha2Nistp256 => "ecdsa-sha2-nistp256",
            KeyType::EcdsaSha2Nistp384 => "ecdsa-sha2-nistp384",
            KeyType::EcdsaSha2Nistp521 => "ecdsa-sha2-nistp521",
            KeyType::SshEd25519 => "ssh-ed25519"
        })
    }
}

impl TryFrom<&str> for KeyType {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use KeyType::*;

        Ok(match value {
            "ssh-rsa" => SshRsa,
            "ecdsa-sha2-nistp256" => EcdsaSha2Nistp256,
            "ecdsa-sha2-nistp384" => EcdsaSha2Nistp384,
            "ecdsa-sha2-nistp521" => EcdsaSha2Nistp521,
            "ssh-ed25519" => SshEd25519,
            _ => bail!("Unknown key type: {}", value)
        })
    }
}
