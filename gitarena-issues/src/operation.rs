use crate::author::Author;
use anyhow::Result;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

/// Operation type as used in `git-bug` json serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OperationType(pub i64);

impl OperationType {
    /// Create bug
    pub const CREATE: Self = Self(1);
    /// Change bug title
    pub const SET_TITLE: Self = Self(2);
    /// Add comment
    pub const ADD_COMMENT: Self = Self(3);
    /// Change status
    pub const SET_STATUS: Self = Self(4);
    /// Change labels
    pub const LABEL_CHANGE: Self = Self(5);
    /// Edit comment
    pub const EDIT_COMMENT: Self = Self(6);
}

/// Bug status as used in `git-bug` json serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BugStatus(pub u8);

impl BugStatus {
    /// Open
    pub const OPEN: Self = Self(1);
    /// Closed
    pub const CLOSED: Self = Self(2);
}

/// Operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Type
    #[serde(rename = "type")]
    pub op_type: OperationType,
    /// Unix timestamp when this operation was created
    pub timestamp: i64,
    /// Random nonce to differentiate operations with the same `op_type`, `timestamp` and `author`
    pub nonce: String,

    /// Bug or comment title, used by [`OperationType::CREATE`] and [`OperationType::SET_TITLE`]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Comment or bug body, used by [`OperationType::CREATE`], [`OperationType::ADD_COMMENT`] and [`OperationType::EDIT_COMMENT`]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Target status, used by [`OperationType::SET_STATUS`]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BugStatus>,

    /// Labels to add, used by [`OperationType::LABEL_CHANGE`]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added: Option<Vec<String>>,

    /// Labels to remove, used by [`OperationType::LABEL_CHANGE`]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed: Option<Vec<String>>,

    /// Target operation ID to edit, used by [`OperationType::EDIT_COMMENT`]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// File attachments, always `null` in current serialization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<serde_json::Value>,

    /// sha256 of the raw json bytes this operation was read from. `None` for operations not read from git
    #[serde(skip)]
    pub(crate) raw_id: Option<String>,
}

impl Operation {
    /// Compute the `git-bug` operation ID as the sha256 hex digest of the serialized operation
    pub fn compute_id(&self) -> Result<String> {
        // re-serializing doesn't reproduce git-bug's bytes (Go escapes `<>&`, other field order, fields we don't model)
        if let Some(raw_id) = &self.raw_id {
            return Ok(raw_id.clone());
        }

        let json = serde_json::to_vec(self)?;

        let hash = Sha256::digest(&json);
        Ok(hex::encode(hash))
    }
}

/// A single commit's worth of operations from one author.
/// Serialized as json into the `ops` blob of a bug commit tree
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "RawOperationPack")]
pub struct OperationPack {
    /// Author
    pub author: Author,
    /// Operations
    pub ops: Vec<Operation>,
}

#[derive(Deserialize)]
struct RawOperationPack {
    author: Author,
    ops: Vec<Box<RawValue>>,
}

#[derive(Deserialize)]
struct CommonFields {
    #[serde(rename = "type")]
    op_type: OperationType,
    timestamp: i64,
    nonce: String,
}

impl TryFrom<RawOperationPack> for OperationPack {
    type Error = serde_json::Error;

    fn try_from(raw: RawOperationPack) -> Result<Self, Self::Error> {
        let ops = raw
            .ops
            .iter()
            .map(|raw_op| {
                let common: CommonFields = serde_json::from_str(raw_op.get())?;

                // unknown types may reuse our field names with other json types, so only parse what we understand
                let mut op = match common.op_type {
                    OperationType::CREATE
                    | OperationType::SET_TITLE
                    | OperationType::ADD_COMMENT
                    | OperationType::SET_STATUS
                    | OperationType::LABEL_CHANGE
                    | OperationType::EDIT_COMMENT => serde_json::from_str(raw_op.get())?,
                    _ => Operation {
                        op_type: common.op_type,
                        timestamp: common.timestamp,
                        nonce: common.nonce,
                        title: None,
                        message: None,
                        status: None,
                        added: None,
                        removed: None,
                        target: None,
                        files: None,
                        raw_id: None,
                    },
                };

                op.raw_id = Some(hex::encode(Sha256::digest(raw_op.get().as_bytes())));
                Ok(op)
            })
            .collect::<Result<_, Self::Error>>()?;

        Ok(OperationPack { author: raw.author, ops })
    }
}

/// Generate a random 20-byte nonce encoded as base64
#[must_use]
pub fn random_nonce() -> String {
    let bytes: [u8; 20] = rand::rng().random();
    base64::encode(bytes)
}
