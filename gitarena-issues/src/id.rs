//! sha256 IDs used by `git-bug` for bugs, operations and identities

use anyhow::{Result, ensure};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt::{self, Debug, Display, Formatter};
use std::str::FromStr;

macro_rules! sha256_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        ///
        /// Displayed, parsed and serialized as 64 lowercase hex characters
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            /// Raw sha256 digest
            #[must_use]
            pub fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                f.write_str(&hex::encode(self.0))
            }
        }

        impl Debug for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}({self})", stringify!($name))
            }
        }

        impl FromStr for $name {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> Result<Self> {
                // uppercase hex would point to a different ref than the canonical lowercase ID
                ensure!(
                    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
                    "invalid {}: {s}",
                    stringify!($name)
                );

                let mut bytes = [0; 32];
                hex::decode_to_slice(s, &mut bytes)?;
                Ok(Self(bytes))
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                String::deserialize(deserializer)?.parse().map_err(D::Error::custom)
            }
        }
    };
}

sha256_id! {
    /// ID of a bug, which is the [`OperationId`] of its initial `CREATE` operation
    BugId
}

impl BugId {
    pub(crate) fn from_create_operation(id: OperationId) -> Self {
        Self(id.0)
    }
}

/// The bug body is the `CREATE` operation, so this targets it in [`crate::ops::edit_comment`]
impl From<BugId> for OperationId {
    fn from(id: BugId) -> Self {
        Self(id.0)
    }
}

sha256_id! {
    /// ID of an operation, which is the sha256 of its json serialization.
    /// Comments are identified by the ID of their `ADD_COMMENT` operation
    OperationId
}

impl OperationId {
    pub(crate) fn digest(data: impl AsRef<[u8]>) -> Self {
        Self(Sha256::digest(data).into())
    }
}

sha256_id! {
    /// ID of an identity, which is the sha256 of its first version blob
    AuthorId
}

impl AuthorId {
    pub(crate) fn digest(data: impl AsRef<[u8]>) -> Self {
        Self(Sha256::digest(data).into())
    }
}
