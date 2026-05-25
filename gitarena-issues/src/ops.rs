use crate::author::Author;
use crate::bug::{Bug, append_operation};
use crate::operation::{BugStatus, Operation, OperationType, random_nonce};
use anyhow::Result;
use chrono::Utc;
use gix::Repository;
use serde_json::Value;
use tracing::instrument;

/// Append an `ADD_COMMENT` operation to a bug
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`gix::Repository`]
/// - `bug`: The bug to comment on
/// - `author`: Author of the comment
/// - `message`: Body of the comment
///
/// # Returns
///
/// - Updated bug
/// - Comment's operation ID
#[instrument(err, skip(git_repo))]
pub fn add_comment(git_repo: &Repository, bug: Bug, author: Author, message: String) -> Result<(Bug, String)> {
    let now = Utc::now().timestamp();

    let op = Operation {
        op_type: OperationType::ADD_COMMENT,
        timestamp: now,
        nonce: random_nonce(),
        title: None,
        message: Some(message),
        status: None,
        added: None,
        removed: None,
        target: None,
        files: Some(Value::Null),
    };

    let op_id = op.compute_id()?;
    let bug = append_operation(git_repo, bug, author, op)?;

    Ok((bug, op_id))
}

/// Append a `SET_TITLE` operation to a bug
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`gix::Repository`]
/// - `bug`: The bug whose title to update
/// - `author`: Author of the change
/// - `new_title`: New title
#[instrument(err, skip(git_repo))]
pub fn set_title(git_repo: &Repository, bug: Bug, author: Author, new_title: String) -> Result<Bug> {
    let now = Utc::now().timestamp();

    let op = Operation {
        op_type: OperationType::SET_TITLE,
        timestamp: now,
        nonce: random_nonce(),
        title: Some(new_title),
        message: None,
        status: None,
        added: None,
        removed: None,
        target: None,
        files: None,
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
#[instrument(err, skip(git_repo))]
pub fn set_status(git_repo: &Repository, bug: Bug, author: Author, status: BugStatus) -> Result<Bug> {
    let now = Utc::now().timestamp();

    let op = Operation {
        op_type: OperationType::SET_STATUS,
        timestamp: now,
        nonce: random_nonce(),
        title: None,
        message: None,
        status: Some(status),
        added: None,
        removed: None,
        target: None,
        files: None,
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
#[instrument(err, skip(git_repo))]
pub fn change_labels(git_repo: &Repository, bug: Bug, author: Author, add: Vec<String>, remove: Vec<String>) -> Result<Bug> {
    let now = Utc::now().timestamp();

    let op = Operation {
        op_type: OperationType::LABEL_CHANGE,
        timestamp: now,
        nonce: random_nonce(),
        title: None,
        message: None,
        status: None,
        added: Some(add),
        removed: Some(remove),
        target: None,
        files: None,
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
/// - `target_op_id`: Operation ID of the comment to edit. Pass the bug's own ID to edit the bug body
/// - `new_message`: Replacement body text
#[instrument(err, skip(git_repo))]
pub fn edit_comment(git_repo: &Repository, bug: Bug, author: Author, target_op_id: String, new_message: String) -> Result<Bug> {
    let now = Utc::now().timestamp();

    let op = Operation {
        op_type: OperationType::EDIT_COMMENT,
        timestamp: now,
        nonce: random_nonce(),
        title: None,
        message: Some(new_message),
        status: None,
        added: None,
        removed: None,
        target: Some(target_op_id),
        files: None,
    };

    append_operation(git_repo, bug, author, op)
}
