use std::io::Cursor;
use crate::user::User;
use anyhow::{Context, Result};
use git2::{Commit, Oid, Repository};
use git_repository::actor::SignatureRef;
use pgp::{Deserializable, SignedPublicKey};
use sqlx::{Executor, Postgres};
use crate::error::GAErrors::Utf8DecodeError;
use crate::extensions::libgit2_to_gitoxide_signature;

pub(crate) fn verify_commit<'e, E: Executor<'e, Database = Postgres>>(user: &User, repo: Repository, commit: &Commit, executor: E) -> Result<bool> {
    let (signature_buffer, content_buffer) = repo.extract_signature(&commit.id(), None)?;
    let signature = signature_buffer.as_str().ok_or(Utf8DecodeError).context("Failed to decode signature")?;
    let content = content_buffer.as_str().ok_or(Utf8DecodeError).context("Failed to decode signature content")?;

    for line in content.lines() {
        if let Some(tree) = line.strip_prefix("tree ") {
            if !validate_tree(commit, tree) {
                return Ok(false);
            }
        } else if let Some(parent) = line.strip_prefix("parent ") {
            if !validate_parent(commit, parent) {
                return Ok(false);
            }
        } else if let Some(author) = line.strip_prefix("author ") {
            if !validate_author(commit, author)? {
                return Ok(false);
            }
        } else {
            break;
        }
    }

    // Verify with pgpg if the signature actually matches the content and uploaded gpg keys
    todo!()
}

fn validate_tree(commit: &Commit, tree_oid_str: &str) -> bool {
    commit.tree_id().to_string().as_str() == tree_oid_str
}

fn validate_parent(commit: &Commit, parent_oid_str: &str) -> bool {
    for oid in commit.parent_ids() {
        if oid.to_string().as_str() == parent_oid_str {
            return true;
        }
    }

    false
}

fn validate_author(commit: &Commit, signature_str: &str) -> Result<bool> {
    let signature_ref = SignatureRef::from_bytes::<()>(signature_str.as_bytes())?;
    let commit_signature = libgit2_to_gitoxide_signature(&commit.author());

    Ok(commit_signature.to_ref() == signature_ref)
}
