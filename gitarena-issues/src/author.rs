use anyhow::{Context, Result, bail, ensure};
use chrono::Utc;
use gix::actor::Signature;
use gix::date::Time;
use gix::date::parse::TimeBuf;
use gix::objs::tree::{Entry, EntryKind};
use gix::objs::{BlobRef, Tree};
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
    pub id: String,
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
    pub fn from_user(user_id: Uuid, username: &str, email: &str, created_at: i64) -> Self {
        let nonce_hash = Sha256::digest(format!("gitarena-identity:{user_id}").as_bytes());
        let nonce = base64::encode(&nonce_hash[..20]);

        let identity_json = build_identity_json(username, email, &nonce, created_at);

        Author {
            id: hex::encode(Sha256::digest(&identity_json)),
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
    pub fn write_identity_if_absent(&self, repo: &Repository) -> Result<()> {
        let ref_name = format!("{IDENTITIES_REF_PREFIX}{}", self.id);

        if repo.find_reference(ref_name.as_str()).is_ok() {
            return Ok(());
        }

        let identity_json = build_identity_json(&self.name, &self.email, &self.nonce, self.created_at);

        let blob_id = repo
            .write_object(BlobRef { data: &identity_json })
            .context("failed to write identity blob")?
            .detach();

        let tree = Tree {
            entries: vec![Entry {
                mode: EntryKind::Blob.into(),
                filename: "version".into(),
                oid: blob_id,
            }],
        };

        let tree_id = repo.write_object(&tree).context("failed to write identity tree")?.detach();

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
        .context("failed to write identity commit")?;

        Ok(())
    }
}

/// Load the username of an author from the git identity ref stored at `refs/identities/<author_id>`,
/// rejecting the identity if it violates the `git-bug` spec
///
/// # Return
///
/// `name` field from the latest identity version
pub fn load_author_name(repo: &Repository, author_id: &str) -> Result<String> {
    let ref_name = format!("{IDENTITIES_REF_PREFIX}{author_id}");
    let reference = repo.find_reference(ref_name.as_str()).context("identity ref not found")?;

    let mut commit_id = reference.id().detach();
    let mut latest: Option<Value> = None;

    loop {
        let commit = repo.find_object(commit_id)?.try_into_commit()?;
        let commit_ref = commit.decode()?;
        let parents: Vec<ObjectId> = commit_ref.parents().collect();

        ensure!(parents.len() <= 1, "identity {author_id} has a merge commit");

        let tree = repo.find_object(commit_ref.tree())?.try_into_tree()?;
        let tree_ref = tree.decode()?;

        let [entry] = tree_ref.entries.as_slice() else {
            bail!("identity commit {commit_id} must have exactly one tree entry");
        };
        ensure!(entry.filename == "version", "identity commit {commit_id} has no version entry");

        let blob = repo.find_object(entry.oid.to_owned())?.try_into_blob()?;
        let value: Value = serde_json::from_slice(&blob.data).context("failed to parse identity blob")?;
        ensure!(
            value["version"] == 2,
            "identity commit {commit_id} has unsupported version {}",
            value["version"]
        );

        latest.get_or_insert(value);

        let Some(parent) = parents.first() else {
            ensure!(
                hex::encode(Sha256::digest(&blob.data)) == author_id,
                "identity {author_id} does not match the ID of its first version"
            );
            break;
        };

        commit_id = *parent;
    }

    let latest = latest.context("identity has no versions")?;
    let name = latest["name"].as_str().context("name field missing from identity blob")?;

    Ok(name.to_owned())
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
    use pretty_assertions::assert_eq;
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
}
