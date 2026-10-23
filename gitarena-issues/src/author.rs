//! `git-bug` identities stored under [`IDENTITIES_REF_PREFIX`]

use crate::bug::{MergeOutcome, update_ref};
use crate::error::Error;
use crate::id::AuthorId;
use chrono::Utc;
use gix::actor::Signature;
use gix::date::Time;
use gix::date::parse::TimeBuf;
use gix::objs::tree::{Entry, EntryKind};
use gix::objs::{BlobRef, Tree};
use gix::refs::Target;
use gix::refs::transaction::PreviousValue;
use gix::{ObjectId, Repository};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Ref location where identities are stored.
/// Build a path to identities by concatenating this variable and the id
pub const IDENTITIES_REF_PREFIX: &str = "refs/identities/";

/// This struct will be mapped to a `git-bug` identity. Construct with [`Author::from_user`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    /// Author IDs in `git-bug` are the sha256 digest of the identity json blob
    pub id: AuthorId,
    /// Username
    #[serde(skip)]
    pub name: String,
    /// E-Mail address
    #[serde(skip)]
    pub email: String,
    /// Nonce is a sha256 digest of the users actual ID (calculated with `gitarena-identity:<uuid>`).
    /// This is used by `git-bug` to differentiate between users that have the same [`name`], [`email`] and [`created_at`].
    #[serde(skip)]
    nonce: String,
    /// Unix timestamp when the user was created
    #[serde(skip)]
    created_at: i64,
}

#[derive(Serialize)]
struct IdentityBlob<'a> {
    version: u8,
    times: Map<String, Value>,
    unix_time: i64,
    name: &'a str,
    email: &'a str,
    nonce: &'a str,
}

impl Author {
    /// Create a new [`Author`]. The passed arguments _should_ be stable so the generated
    /// `git-bug` ID and the generated `nonce` does not differ between operations.
    ///
    /// # Arguments
    ///
    /// - `user_id`: UUID of the user
    /// - `username`: Username of the user
    /// - `email`: E-mail of the user. Note that this will be visible to any `git-bug` user so you may consider using an anonymized email like GitArena does.
    /// - `created_at`: Unix timestamp when the user was created
    #[must_use]
    pub fn from_user(user_id: Uuid, username: &str, email: &str, created_at: i64) -> Self {
        let nonce_hash = Sha256::digest(format!("gitarena-identity:{user_id}").as_bytes());
        let nonce = base64::encode(&nonce_hash[..20]);

        let identity_json = build_identity_json(username, email, &nonce, created_at);

        Author {
            id: AuthorId::digest(&identity_json),
            name: username.to_owned(),
            email: email.to_owned(),
            nonce,
            created_at,
        }
    }

    /// Write the `git-bug` identity into the identities ref, if the user has not yet been written.
    ///
    /// # Arguments
    ///
    /// - `repo`: Handle to a [`gix::Repository`]
    pub fn write_identity_if_absent(&self, repo: &Repository) -> Result<(), Error> {
        let ref_name = format!("{IDENTITIES_REF_PREFIX}{}", self.id);

        if repo.find_reference(ref_name.as_str()).is_ok() {
            return Ok(());
        }

        let identity_json = build_identity_json(&self.name, &self.email, &self.nonce, self.created_at);

        let blob_id = repo.write_object(BlobRef { data: &identity_json }).map_err(Error::git)?.detach();

        let tree = Tree {
            entries: vec![Entry {
                mode: EntryKind::Blob.into(),
                filename: "version".into(),
                oid: blob_id,
            }],
        };

        let tree_id = repo.write_object(&tree).map_err(Error::git)?.detach();

        let now = Utc::now();

        let sig = Signature {
            name: self.name.as_bytes().into(),
            email: self.email.as_bytes().into(),
            time: Time {
                seconds: now.timestamp(),
                offset: 0,
            },
        };

        let mut time_buffer_committer = TimeBuf::default();
        let mut time_buffer_author = TimeBuf::default();

        repo.commit_as(
            sig.to_ref(&mut time_buffer_committer),
            sig.to_ref(&mut time_buffer_author),
            ref_name.as_str(),
            "",
            tree_id,
            Vec::<ObjectId>::new(),
        )
        .map_err(Error::git)?;

        Ok(())
    }
}

/// Load the username of an author from the git identity ref stored at `refs/identities/<author_id>`,
/// rejecting the identity if it violates the `git-bug` spec
///
/// # Return
///
/// `name` field from the latest identity version
pub fn load_author_name(repo: &Repository, author_id: &AuthorId) -> Result<String, Error> {
    let ref_name = format!("{IDENTITIES_REF_PREFIX}{author_id}");
    let reference = repo
        .try_find_reference(ref_name.as_str())
        .map_err(Error::git)?
        .ok_or(Error::IdentityNotFound(*author_id))?;

    let (_, latest) = read_identity(repo, author_id, reference.id().detach())?;
    let name = latest["name"].as_str().ok_or_else(|| Error::InvalidIdentity {
        id: *author_id,
        reason: "name field missing from latest version".to_owned(),
    })?;

    Ok(name.to_owned())
}

/// Merge `remote_tip` into the local identity ref, e.g. after fetching `refs/remotes/<remote>/identities/<id>`.
/// Identities can only be fast-forwarded (spec identity.md §6), so diverged histories are an error.
///
/// # Arguments
///
/// - `repo`: Handle to a [`Repository`]
/// - `author_id`: ID of the identity
/// - `remote_tip`: Commit to merge, must already be in the object database
pub fn merge_identity(repo: &Repository, author_id: &AuthorId, remote_tip: ObjectId) -> Result<MergeOutcome, Error> {
    let (remote_chain, _) = read_identity(repo, author_id, remote_tip)?;
    let ref_name = format!("{IDENTITIES_REF_PREFIX}{author_id}");

    loop {
        let local_tip = repo
            .try_find_reference(ref_name.as_str())
            .map_err(Error::git)?
            .map(|reference| reference.id().detach());

        let (outcome, expected) = match local_tip {
            None => (MergeOutcome::New, PreviousValue::MustNotExist),
            Some(local_tip) if local_tip == remote_tip => return Ok(MergeOutcome::Identical),
            Some(local_tip) if remote_chain.contains(&local_tip) => (MergeOutcome::FastForward, PreviousValue::MustExistAndMatch(Target::Object(local_tip))),
            Some(local_tip) => {
                let (local_chain, _) = read_identity(repo, author_id, local_tip)?;
                if !local_chain.contains(&remote_tip) {
                    return Err(Error::IdentityDiverged(*author_id));
                }

                return Ok(MergeOutcome::LocalAhead);
            }
        };

        if update_ref(repo, &ref_name, remote_tip, expected)? {
            return Ok(outcome);
        }
    }
}

fn read_identity(repo: &Repository, author_id: &AuthorId, tip: ObjectId) -> Result<(Vec<ObjectId>, Value), Error> {
    let invalid = |reason: String| Error::InvalidIdentity { id: *author_id, reason };

    let mut commit_id = tip;
    let mut chain = Vec::new();
    let mut latest: Option<Value> = None;

    loop {
        let commit = repo.find_object(commit_id).map_err(Error::git)?.try_into_commit().map_err(Error::git)?;
        let commit_ref = commit.decode().map_err(Error::git)?;
        let parents: Vec<ObjectId> = commit_ref.parents().collect();

        if parents.len() > 1 {
            return Err(invalid(format!("commit {commit_id} is a merge commit")));
        }

        let tree = repo.find_object(commit_ref.tree()).map_err(Error::git)?.try_into_tree().map_err(Error::git)?;
        let tree_ref = tree.decode().map_err(Error::git)?;

        let [entry] = tree_ref.entries.as_slice() else {
            return Err(invalid(format!("commit {commit_id} must have exactly one tree entry")));
        };

        if entry.filename != "version" {
            return Err(invalid(format!("commit {commit_id} has no version entry")));
        }

        let blob = repo
            .find_object(entry.oid.to_owned())
            .map_err(Error::git)?
            .try_into_blob()
            .map_err(Error::git)?;
        let value: Value = serde_json::from_slice(&blob.data).map_err(|err| invalid(format!("commit {commit_id} has an invalid version blob: {err}")))?;

        if value["version"] != 2 {
            return Err(invalid(format!("commit {commit_id} has unsupported version {}", value["version"])));
        }

        chain.push(commit_id);
        latest.get_or_insert(value);

        let Some(parent) = parents.first() else {
            if AuthorId::digest(&blob.data) != *author_id {
                return Err(invalid("does not match the ID of its first version".to_owned()));
            }
            break;
        };

        commit_id = *parent;
    }

    let latest = latest.ok_or_else(|| invalid("has no versions".to_owned()))?;
    Ok((chain, latest))
}

fn build_identity_json(name: &str, email: &str, nonce: &str, unix_time: i64) -> Vec<u8> {
    let blob = IdentityBlob {
        version: 2,
        times: Map::new(),
        unix_time,
        name,
        email,
        nonce,
    };

    serde_json::to_vec(&blob).expect("identity json to be able to be serialized")
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use pretty_assertions::{assert_eq, assert_matches};
    use tempfile::tempdir;

    #[test]
    fn load_author_name_returns_username() -> Result<()> {
        let dir = tempdir()?;
        let repo = gix::init(dir.path())?;

        let author = Author::from_user(Uuid::nil(), "testuser", "testuser@gitarena.local", 1_700_000_000);
        author.write_identity_if_absent(&repo)?;

        let name = load_author_name(&repo, &author.id)?;
        assert_eq!(name, "testuser");

        Ok(())
    }

    fn version(repo: &Repository, name: &str, parent: ObjectId) -> Result<ObjectId> {
        let blob = format!(r#"{{"version":2,"times":{{}},"unix_time":1,"name":"{name}","nonce":"bm9uY2U="}}"#);
        let blob_id = repo.write_blob(blob)?.detach();

        let tree = Tree {
            entries: vec![Entry {
                mode: EntryKind::Blob.into(),
                filename: "version".into(),
                oid: blob_id,
            }],
        };
        let tree_id = repo.write_object(&tree)?.detach();

        let sig = Signature {
            name: "test".into(),
            email: "test@gitarena.local".into(),
            time: Time::default(),
        };

        let mut time_buffer_committer = TimeBuf::default();
        let mut time_buffer_author = TimeBuf::default();

        let commit = repo.new_commit_as(
            sig.to_ref(&mut time_buffer_committer),
            sig.to_ref(&mut time_buffer_author),
            "",
            tree_id,
            [parent],
        )?;

        Ok(commit.id)
    }

    #[test]
    fn merge_identity_only_fast_forwards() -> Result<()> {
        let dir = tempdir()?;
        let repo = gix::init(dir.path())?;

        let author = Author::from_user(Uuid::nil(), "testuser", "testuser@gitarena.local", 1_700_000_000);
        author.write_identity_if_absent(&repo)?;

        let ref_name = format!("{IDENTITIES_REF_PREFIX}{}", author.id);
        let first = repo.find_reference(ref_name.as_str())?.id().detach();
        let renamed = version(&repo, "renamed", first)?;
        let diverged = version(&repo, "diverged", first)?;

        assert_eq!(merge_identity(&repo, &author.id, first)?, MergeOutcome::Identical);
        assert_eq!(merge_identity(&repo, &author.id, renamed)?, MergeOutcome::FastForward);
        assert_eq!(load_author_name(&repo, &author.id)?, "renamed");
        assert_eq!(merge_identity(&repo, &author.id, first)?, MergeOutcome::LocalAhead);

        let err = merge_identity(&repo, &author.id, diverged).expect_err("diverged identity");
        assert_matches!(err, Error::IdentityDiverged(id) if id == author.id);

        let err = merge_identity(&repo, &"0".repeat(64).parse()?, renamed).expect_err("identity with another ID");
        assert_matches!(err, Error::InvalidIdentity { reason, .. } if reason.contains("does not match"));

        repo.find_reference(ref_name.as_str())?.delete()?;
        assert_eq!(merge_identity(&repo, &author.id, renamed)?, MergeOutcome::New);

        Ok(())
    }
}
