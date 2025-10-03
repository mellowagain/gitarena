use std::fmt;
use std::fmt::{Display, Formatter};

use anyhow::{bail, Error};
use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Type, Debug, Deserialize, Serialize)]
#[sqlx(type_name = "ssh_key_type", rename_all = "kebab-case")]
pub enum KeyType {
    SshRsa,
    EcdsaSha2Nistp256,
    EcdsaSha2Nistp384,
    EcdsaSha2Nistp521,
    SshEd25519,
}

impl Display for KeyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use KeyType::*;

        f.write_str(match self {
            SshRsa => "ssh-rsa",
            EcdsaSha2Nistp256 => "ecdsa-sha2-nistp256",
            EcdsaSha2Nistp384 => "ecdsa-sha2-nistp384",
            EcdsaSha2Nistp521 => "ecdsa-sha2-nistp521",
            SshEd25519 => "ssh-ed25519",
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
            _ => bail!("Unknown key type: {}", value),
        })
    }
}
