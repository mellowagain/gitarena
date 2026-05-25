use crate::author::Author;
use anyhow::Result;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Operation type as used in `git-bug` json serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OperationType(pub u8);

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
}

impl Operation {
    /// Compute the `git-bug` operation ID as the sha256 hex digest of the serialized operation
    pub fn compute_id(&self) -> Result<String> {
        let json = serde_json::to_vec(self)?;

        let hash = Sha256::digest(&json);
        Ok(hex::encode(hash))
    }
}

/// A single commit's worth of operations from one author.
/// Serialized as json into the `ops` blob of a bug commit tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationPack {
    /// Author
    pub author: Author,
    /// Operations
    pub ops: Vec<Operation>,
}

/// Generate a random 20-byte nonce encoded as base64
#[must_use]
pub fn random_nonce() -> String {
    let bytes: [u8; 20] = rand::rng().random();
    base64::encode(bytes)
}
