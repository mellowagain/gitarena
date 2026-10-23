//! Operations that can be appended to an existing [`Bug`]

use crate::author::Author;
use crate::bug::{Bug, append_operation};
use crate::error::Error;
use crate::id::OperationId;
use crate::operation::{BugStatus, Operation, OperationType, random_nonce};
use chrono::Utc;
use gix::Repository;
use serde_json::Value;
#[cfg(feature = "tracing")]
use tracing::instrument;

/// Result of [`add_comment`]
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AddedComment {
    /// Updated bug
    pub bug: Bug,
    /// Operation ID of the new comment
    pub comment_id: OperationId,
}

/// Append an `ADD_COMMENT` operation to a bug
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`gix::Repository`]
/// - `bug`: The bug to comment on
/// - `author`: Author of the comment
/// - `message`: Body of the comment
#[cfg_attr(feature = "tracing", instrument(err, skip(git_repo)))]
pub fn add_comment(git_repo: &Repository, bug: Bug, author: Author, message: String) -> Result<AddedComment, Error> {
    let now = Utc::now().timestamp();

    let op = Operation {
        op_type: OperationType::ADD_COMMENT,
        timestamp: now,
        nonce: random_nonce(),
        title: None,
        was: None,
        message: Some(message),
        status: None,
        added: None,
        removed: None,
        target: None,
        files: Some(Value::Null),
        raw_id: None,
    };

    let comment_id = op.compute_id();
    let bug = append_operation(git_repo, bug, author, op)?;

    Ok(AddedComment { bug, comment_id })
}

/// Append a `SET_TITLE` operation to a bug
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`gix::Repository`]
/// - `bug`: The bug whose title to update
/// - `author`: Author of the change
/// - `new_title`: New title
#[cfg_attr(feature = "tracing", instrument(err, skip(git_repo)))]
pub fn set_title(git_repo: &Repository, bug: Bug, author: Author, new_title: String) -> Result<Bug, Error> {
    let now = Utc::now().timestamp();
    let was = bug.snapshot().title;

    let op = Operation {
        op_type: OperationType::SET_TITLE,
        timestamp: now,
        nonce: random_nonce(),
        title: Some(new_title),
        was: Some(was),
        message: None,
        status: None,
        added: None,
        removed: None,
        target: None,
        files: None,
        raw_id: None,
    };

    append_operation(git_repo, bug, author, op)
}

/// Append a `SET_STATUS` operation to a bug
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`Repository`]
/// - `bug`: The bug to update
/// - `author`: Author of the change
/// - `status`: The new [`BugStatus`] to set
#[cfg_attr(feature = "tracing", instrument(err, skip(git_repo)))]
pub fn set_status(git_repo: &Repository, bug: Bug, author: Author, status: BugStatus) -> Result<Bug, Error> {
    let now = Utc::now().timestamp();

    let op = Operation {
        op_type: OperationType::SET_STATUS,
        timestamp: now,
        nonce: random_nonce(),
        title: None,
        was: None,
        message: None,
        status: Some(status),
        added: None,
        removed: None,
        target: None,
        files: None,
        raw_id: None,
    };

    append_operation(git_repo, bug, author, op)
}

/// Append a `LABEL_CHANGE` operation to a bug
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`Repository`]
/// - `bug`: The bug to update
/// - `author`: Author of the change
/// - `add`: Labels to add
/// - `remove`: Labels to remove
#[cfg_attr(feature = "tracing", instrument(err, skip(git_repo)))]
pub fn change_labels(git_repo: &Repository, bug: Bug, author: Author, add: Vec<String>, remove: Vec<String>) -> Result<Bug, Error> {
    let now = Utc::now().timestamp();

    let op = Operation {
        op_type: OperationType::LABEL_CHANGE,
        timestamp: now,
        nonce: random_nonce(),
        title: None,
        was: None,
        message: None,
        status: None,
        added: Some(add),
        removed: Some(remove),
        target: None,
        files: None,
        raw_id: None,
    };

    append_operation(git_repo, bug, author, op)
}

/// Append a `EDIT_COMMENT` operation to a bug
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`Repository`]
/// - `bug`: The bug containing the comment
/// - `author`: Author of the edit
/// - `target_op_id`: Operation ID of the comment to edit. Pass `OperationId::from(bug.id)` to edit the bug body
/// - `new_message`: Replacement body text
#[cfg_attr(feature = "tracing", instrument(err, skip(git_repo)))]
pub fn edit_comment(git_repo: &Repository, bug: Bug, author: Author, target_op_id: OperationId, new_message: String) -> Result<Bug, Error> {
    let now = Utc::now().timestamp();

    let op = Operation {
        op_type: OperationType::EDIT_COMMENT,
        timestamp: now,
        nonce: random_nonce(),
        title: None,
        was: None,
        message: Some(new_message),
        status: None,
        added: None,
        removed: None,
        target: Some(target_op_id),
        files: Some(Value::Null),
        raw_id: None,
    };

    append_operation(git_repo, bug, author, op)
}
