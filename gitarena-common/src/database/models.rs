use std::fmt;
use std::fmt::{Display, Formatter};

use anyhow::{Error, bail};
use russh::keys::{Algorithm, EcdsaCurve};
use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Type, Debug, Deserialize, Serialize, Copy, Clone)]
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
            _ => bail!("Unknown key type: {value}"),
        })
    }
}

impl TryFrom<&Algorithm> for KeyType {
    type Error = Error;

    fn try_from(value: &Algorithm) -> Result<Self, Self::Error> {
        use KeyType::*;

        Ok(match value {
            Algorithm::Dsa => bail!("DSA keys are unsupported"),
            Algorithm::Ecdsa { curve } => match curve {
                EcdsaCurve::NistP256 => EcdsaSha2Nistp256,
                EcdsaCurve::NistP384 => EcdsaSha2Nistp384,
                EcdsaCurve::NistP521 => EcdsaSha2Nistp521,
            },
            Algorithm::Rsa { .. } => SshRsa,
            Algorithm::SkEcdsaSha2NistP256 => EcdsaSha2Nistp256,
            Algorithm::Ed25519 | Algorithm::SkEd25519 => SshEd25519,
            Algorithm::Other(name) => bail!("Unknown algorithm: {}", name.as_str()),
            _ => bail!("Unknown algorithm"),
        })
    }
}
