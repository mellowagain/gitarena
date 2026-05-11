use std::fmt;
use std::fmt::{Display, Formatter};

use anyhow::{Error, bail};
use russh::keys::{Algorithm, EcdsaCurve, HashAlg};
use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Type, Debug, Deserialize, Serialize, Copy, Clone)]
#[sqlx(type_name = "ssh_key_type", rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum KeyType {
    SshRsa,
    EcdsaSha2Nistp256,
    EcdsaSha2Nistp384,
    EcdsaSha2Nistp521,
    SshEd25519,
    #[sqlx(rename = "sk-ssh-ed25519@openssh.com")]
    #[serde(rename = "sk-ssh-ed25519@openssh.com")]
    SkSshEd25519,
    #[sqlx(rename = "sk-ecdsa-sha2-nistp256@openssh.com")]
    #[serde(rename = "sk-ecdsa-sha2-nistp256@openssh.com")]
    SkEcdsaSha2Nistp256,
    #[sqlx(rename = "rsa-sha2-256")]
    RsaSha2_256,
    #[sqlx(rename = "rsa-sha2-512")]
    RsaSha2_512,
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
            SkSshEd25519 => "sk-ssh-ed25519@openssh.com",
            SkEcdsaSha2Nistp256 => "sk-ecdsa-sha2-nistp256@openssh.com",
            RsaSha2_256 => "rsa-sha2-256",
            RsaSha2_512 => "rsa-sha2-512",
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
            "sk-ssh-ed25519@openssh.com" => SkSshEd25519,
            "sk-ecdsa-sha2-nistp256@openssh.com" => SkEcdsaSha2Nistp256,
            "rsa-sha2-256" => RsaSha2_256,
            "rsa-sha2-512" => RsaSha2_512,
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
            Algorithm::Rsa { hash: Some(HashAlg::Sha256) } => RsaSha2_256,
            Algorithm::Rsa { hash: Some(HashAlg::Sha512) } => RsaSha2_512,
            Algorithm::Rsa { hash: None } | Algorithm::Rsa { .. } => SshRsa,
            Algorithm::SkEcdsaSha2NistP256 => SkEcdsaSha2Nistp256,
            Algorithm::Ed25519 => SshEd25519,
            Algorithm::SkEd25519 => SkSshEd25519,
            Algorithm::Other(name) => bail!("Unknown algorithm: {}", name.as_str()),
            _ => bail!("Unknown algorithm"),
        })
    }
}
