//! Creating, loading and merging bugs stored under [`BUGS_REF_PREFIX`]

use crate::author::Author;
use crate::clock::{self, BUGS_CREATE, BUGS_EDIT};
use crate::error::Error;
use crate::id::{BugId, OperationId};
use crate::operation::{BugStatus, Operation, OperationPack, OperationType, random_nonce};
use chrono::Utc;
use gix::actor::Signature;
use gix::bstr::ByteSlice;
use gix::date::parse::TimeBuf;
use gix::objs::Tree;
use gix::objs::tree::{Entry, EntryKind};
use gix::refs::Target;
use gix::refs::file::transaction::prepare::Error as PrepareError;
use gix::refs::transaction::PreviousValue;
use gix::{ObjectId, Repository};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
#[cfg(feature = "tracing")]
use tracing::instrument;

/// Ref location where bugs are stored.
/// Build a path to a bug by concatenating this variable and the bug id
pub const BUGS_REF_PREFIX: &str = "refs/bugs/";

/// Bug format version written to and accepted from the `version-<N>` tree entry
pub const FORMAT_VERSION: u32 = 4;

/// Largest edit-clock increase allowed between a non-merge commit and its parent (spec dag-entity.md §4)
pub const MAX_CLOCK_JUMP: u64 = 1_000_000;

/// In-memory representation of a `git-bug` bug with all its operation packs loaded.
/// Load from a repository with [`load_bug`] or create with [`create_bug`]
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Bug {
    /// ID of the bug, derived from its initial CREATE operation
    pub id: BugId,
    /// Operation packs, one per commit
    pub packs: Vec<OperationPack>,
    /// Creation clock counter, encoded as a filename in the first commit
    pub create_clock: u64,
    /// Edit clock counter, incremented with each new operation commit
    pub edit_clock: u64,
}

impl Bug {
    /// Replay all operation packs and return the current state of the bug
    #[must_use]
    pub fn snapshot(&self) -> BugSnapshot {
        let mut title = String::new();
        let mut body = String::new();
        let mut status = BugStatus::OPEN;
        let mut labels: Vec<String> = Vec::new();
        let mut comments: Vec<CommentSnapshot> = Vec::new();
        let mut created_at = 0_i64;
        let mut created = false;

        for pack in &self.packs {
            for op in &pack.ops {
                match op.op_type {
                    OperationType::CREATE if !created => {
                        created = true;
                        op.title.as_deref().unwrap_or_default().clone_into(&mut title);
                        op.message.as_deref().unwrap_or_default().clone_into(&mut body);
                        created_at = op.timestamp;
                    }
                    OperationType::SET_TITLE => {
                        if let Some(t) = &op.title {
                            title.clone_from(t);
                        }
                    }
                    OperationType::ADD_COMMENT => {
                        comments.push(CommentSnapshot {
                            id: op.compute_id(),
                            author: pack.author.clone(),
                            body: op.message.as_deref().unwrap_or_default().to_owned(),
                            created_at: op.timestamp,
                            edited_at: None,
                        });
                    }
                    OperationType::SET_STATUS => {
                        if let Some(s) = op.status {
                            status = s;
                        }
                    }
                    OperationType::LABEL_CHANGE => {
                        if let Some(added) = &op.added {
                            for label in added {
                                if !labels.contains(label) {
                                    labels.push(label.clone());
                                }
                            }
                        }

                        if let Some(removed) = &op.removed {
                            labels.retain(|l| !removed.contains(l));
                        }

                        labels.sort();
                    }
                    OperationType::EDIT_COMMENT => {
                        if let (Some(target), Some(new_body)) = (&op.target, &op.message) {
                            if *target == OperationId::from(self.id) {
                                body.clone_from(new_body);
                            } else if let Some(comment) = comments.iter_mut().find(|c| c.id == *target) {
                                comment.body.clone_from(new_body);
                                comment.edited_at = Some(op.timestamp);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        BugSnapshot {
            id: self.id,
            title,
            body,
            status,
            labels,
            comments,
            created_at,
        }
    }
}

/// Current state of a bug derived by replaying its operation packs
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct BugSnapshot {
    /// ID of the bug
    pub id: BugId,
    /// Title
    pub title: String,
    /// Body
    pub body: String,
    /// Status
    pub status: BugStatus,
    /// Labels
    pub labels: Vec<String>,
    /// Comments
    pub comments: Vec<CommentSnapshot>,
    /// Unix timestamp when the bug was created
    pub created_at: i64,
}

/// A single comment in its current state
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CommentSnapshot {
    /// ID of the `ADD_COMMENT` operation
    pub id: OperationId,
    /// Author
    pub author: Author,
    /// Body
    pub body: String,
    /// Unix timestamp when the comment was created
    pub created_at: i64,
    /// Unix timestamp when the comment was last edited, if any
    pub edited_at: Option<i64>,
}

/// Create a new bug and write it as the first commit under [`BUGS_REF_PREFIX`].
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`gix::Repository`]
/// - `author`: Author of the bug
/// - `title`: Initial title
/// - `body`: Initial body text
#[cfg_attr(feature = "tracing", instrument(err, skip(git_repo)))]
pub fn create_bug(git_repo: &Repository, author: Author, title: String, body: String) -> Result<Bug, Error> {
    let now = Utc::now().timestamp();
    let create_clock = clock::increment(git_repo, BUGS_CREATE)?;
    let edit_clock = clock::increment(git_repo, BUGS_EDIT)?;

    let create_op = Operation {
        op_type: OperationType::CREATE,
        timestamp: now,
        nonce: random_nonce(),
        title: Some(title),
        was: None,
        message: Some(body),
        status: None,
        added: None,
        removed: None,
        target: None,
        files: Some(serde_json::Value::Null),
        raw_id: None,
    };

    let bug_id = BugId::from_create_operation(create_op.compute_id());

    let pack = OperationPack { author, ops: vec![create_op] };

    pack.author.write_identity_if_absent(git_repo)?;

    let commit_id = write_pack_as_commit(git_repo, &pack, create_clock, edit_clock, Vec::new())?;
    let ref_name = format!("{BUGS_REF_PREFIX}{bug_id}");
    if !update_ref(git_repo, &ref_name, commit_id, PreviousValue::MustNotExist)? {
        return Err(Error::BugAlreadyExists(bug_id));
    }

    Ok(Bug {
        id: bug_id,
        packs: vec![pack],
        create_clock,
        edit_clock,
    })
}

/// Append an operation to an existing bug as a new commit
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`Repository`]
/// - `bug`: The bug to append to
/// - `author`: Author of the operation
/// - `op`: Operation to append
#[cfg_attr(feature = "tracing", instrument(err, skip(git_repo)))]
pub fn append_operation(git_repo: &Repository, mut bug: Bug, author: Author, op: Operation) -> Result<Bug, Error> {
    // a clock-spammed bug elsewhere in the repo must not push this bug past the jump limit
    let edit_clock = clock::increment(git_repo, BUGS_EDIT)?.min(bug.edit_clock + MAX_CLOCK_JUMP);
    let ref_name = format!("{BUGS_REF_PREFIX}{}", bug.id);
    let current_tip = find_ref_target(git_repo, &ref_name)?.ok_or(Error::BugNotFound(bug.id))?;

    author.write_identity_if_absent(git_repo)?;

    let pack = OperationPack { author, ops: vec![op] };
    let commit_id = write_pack_as_commit(git_repo, &pack, 0, edit_clock, vec![current_tip])?;

    if !update_ref(git_repo, &ref_name, commit_id, PreviousValue::MustExistAndMatch(Target::Object(current_tip)))? {
        merge_bug(git_repo, &bug.id, commit_id, &pack.author)?;
        return load_bug(git_repo, &bug.id);
    }

    bug.packs.push(pack);
    bug.edit_clock = edit_clock;

    Ok(bug)
}

fn write_pack_as_commit(git_repo: &Repository, pack: &OperationPack, create_clock: u64, edit_clock: u64, parents: Vec<ObjectId>) -> Result<ObjectId, Error> {
    let ops_json = serde_json::to_vec(pack).expect("operation pack to be able to be serialized");

    let ops_blob_id = git_repo.write_object(gix::objs::BlobRef { data: &ops_json }).map_err(Error::git)?.detach();

    let empty_blob_id = ObjectId::empty_blob(git_repo.object_hash());

    let mut entries = vec![
        Entry {
            mode: EntryKind::Blob.into(),
            filename: "ops".into(),
            oid: ops_blob_id,
        },
        Entry {
            mode: EntryKind::Blob.into(),
            filename: format!("edit-clock-{edit_clock}").into(),
            oid: empty_blob_id,
        },
        Entry {
            mode: EntryKind::Blob.into(),
            filename: format!("version-{FORMAT_VERSION}").into(),
            oid: empty_blob_id,
        },
    ];

    if create_clock > 0 {
        entries.push(Entry {
            mode: EntryKind::Blob.into(),
            filename: format!("create-clock-{create_clock}").into(),
            oid: empty_blob_id,
        });
    }

    entries.sort_by(|a, b| a.filename.cmp(&b.filename));

    let tree = Tree { entries };
    let tree_id = git_repo.write_object(&tree).map_err(Error::git)?.detach();

    let now = Utc::now();

    let sig = Signature {
        name: pack.author.name.as_bytes().into(),
        email: pack.author.email.as_bytes().into(),
        time: gix::date::Time {
            seconds: now.timestamp(),
            offset: 0,
        },
    };

    let mut time_buffer_committer = TimeBuf::default();
    let mut time_buffer_author = TimeBuf::default();

    let commit = git_repo
        .new_commit_as(
            sig.to_ref(&mut time_buffer_committer),
            sig.to_ref(&mut time_buffer_author),
            "",
            tree_id,
            parents,
        )
        .map_err(Error::git)?;

    Ok(commit.id)
}

/// Point `ref_name` at `new_tip` if it still matches `expected`
///
/// # Returns
///
/// `false` if the ref was changed concurrently and `expected` no longer matches
pub(crate) fn update_ref(git_repo: &Repository, ref_name: &str, new_tip: ObjectId, expected: PreviousValue) -> Result<bool, Error> {
    // gix reads the ref before taking its lock, so without this concurrent updates overwrite each other
    let lock_dir = git_repo.git_dir().join("git-bug");
    fs::create_dir_all(&lock_dir).map_err(Error::RefLock)?;

    let lock = File::create(lock_dir.join("refs.lock")).map_err(Error::RefLock)?;
    lock.lock().map_err(Error::RefLock)?;

    match git_repo.reference(ref_name, new_tip, expected, "") {
        Ok(_) => Ok(true),
        Err(gix::reference::edit::Error::FileTransactionPrepare(
            PrepareError::ReferenceOutOfDate { .. } | PrepareError::MustNotExist { .. } | PrepareError::MustExist { .. },
        )) => Ok(false),
        Err(err) => Err(Error::git(err)),
    }
}

/// Result of merging a remote tip into a local entity (spec dag-entity.md §10)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MergeOutcome {
    /// Entity did not exist locally and was created at the remote tip
    New,
    /// Local and remote tip are the same commit
    Identical,
    /// Remote tip is an ancestor of the local tip
    LocalAhead,
    /// Local tip was an ancestor of the remote tip and got advanced to it
    FastForward,
    /// Histories diverged and were joined with a merge commit
    Merged,
}

/// Merge `remote_tip` into the local bug ref, e.g. after fetching `refs/remotes/<remote>/bugs/<id>`.
/// The remote history is validated like [`load_bug`] does before the local ref is touched.
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`Repository`]
/// - `bug_id`: ID of the bug
/// - `remote_tip`: Commit to merge, must already be in the object database
/// - `author`: Author of the merge commit if the histories diverged
#[cfg_attr(feature = "tracing", instrument(err, skip(git_repo)))]
pub fn merge_bug(git_repo: &Repository, bug_id: &BugId, remote_tip: ObjectId, author: &Author) -> Result<MergeOutcome, Error> {
    let remote = read_dag(git_repo, bug_id, remote_tip)?;
    let remote_commits: HashSet<ObjectId> = remote.keys().copied().collect();
    build_bug(git_repo, bug_id, remote)?;

    let ref_name = format!("{BUGS_REF_PREFIX}{bug_id}");

    loop {
        let local_tip = git_repo
            .try_find_reference(ref_name.as_str())
            .map_err(Error::git)?
            .map(|reference| reference.id().detach());

        let (outcome, new_tip, expected) = match local_tip {
            None => (MergeOutcome::New, remote_tip, PreviousValue::MustNotExist),
            Some(local_tip) if local_tip == remote_tip => return Ok(MergeOutcome::Identical),
            Some(local_tip) if remote_commits.contains(&local_tip) => (
                MergeOutcome::FastForward,
                remote_tip,
                PreviousValue::MustExistAndMatch(Target::Object(local_tip)),
            ),
            Some(local_tip) => {
                let local = read_dag(git_repo, bug_id, local_tip)?;

                if local.contains_key(&remote_tip) {
                    return Ok(MergeOutcome::LocalAhead);
                }

                build_bug(git_repo, bug_id, local)?;
                author.write_identity_if_absent(git_repo)?;

                let pack = OperationPack {
                    author: author.clone(),
                    ops: Vec::new(),
                };

                let edit_clock = clock::increment(git_repo, BUGS_EDIT)?;
                let merge_commit = write_pack_as_commit(git_repo, &pack, 0, edit_clock, vec![local_tip, remote_tip])?;

                (MergeOutcome::Merged, merge_commit, PreviousValue::MustExistAndMatch(Target::Object(local_tip)))
            }
        };

        if update_ref(git_repo, &ref_name, new_tip, expected)? {
            return Ok(outcome);
        }
    }
}

fn find_ref_target(git_repo: &Repository, ref_name: &str) -> Result<Option<ObjectId>, Error> {
    let reference = git_repo.try_find_reference(ref_name).map_err(Error::git)?;
    Ok(reference.map(|reference| reference.id().detach()))
}

/// Load a bug from the repository by its hex ID, rejecting it if it violates the `git-bug` spec.
/// Clock values of the bug are witnessed in the repository's Lamport clocks.
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`Repository`]
/// - `bug_id`: ID of the bug to load
pub fn load_bug(git_repo: &Repository, bug_id: &BugId) -> Result<Bug, Error> {
    let ref_name = format!("{BUGS_REF_PREFIX}{bug_id}");
    let tip = find_ref_target(git_repo, &ref_name)?.ok_or(Error::BugNotFound(*bug_id))?;

    build_bug(git_repo, bug_id, read_dag(git_repo, bug_id, tip)?)
}

fn build_bug(git_repo: &Repository, bug_id: &BugId, commits: HashMap<ObjectId, BugCommit>) -> Result<Bug, Error> {
    let invalid = |reason: String| Error::InvalidBug { id: *bug_id, reason };

    let mut root = None;

    for (id, commit) in &commits {
        if commit.parents.is_empty() {
            if root.replace(commit).is_some() {
                return Err(invalid("has more than one root commit".to_owned()));
            }
            continue;
        }

        let is_merge = commit.parents.len() > 1;
        if is_merge && !commit.pack.ops.is_empty() {
            return Err(invalid(format!("merge commit {id} carries operations")));
        }

        for parent in &commit.parents {
            let parent_clock = commits[parent].edit_clock;

            if commit.edit_clock <= parent_clock {
                return Err(invalid(format!("commit {id} does not advance the edit-clock")));
            }

            if !is_merge && commit.edit_clock - parent_clock > MAX_CLOCK_JUMP {
                return Err(invalid(format!("commit {id} jumps the edit-clock by more than {MAX_CLOCK_JUMP}")));
            }
        }
    }

    let root = root.ok_or_else(|| invalid("has no root commit".to_owned()))?;

    let create_clock = root
        .create_clock
        .filter(|clock| *clock > 0)
        .ok_or_else(|| invalid("root commit has no create-clock".to_owned()))?;

    let create_op = root.pack.ops.first().ok_or_else(|| invalid("root commit has no operations".to_owned()))?;

    if BugId::from_create_operation(create_op.compute_id()) != *bug_id {
        return Err(invalid("does not match the ID of its first operation".to_owned()));
    }

    let mut commits: Vec<BugCommit> = commits.into_values().collect();
    commits.sort_by(|a, b| a.edit_clock.cmp(&b.edit_clock).then_with(|| a.pack_id.cmp(&b.pack_id)));

    let mut op_ids = HashSet::new();

    for op in commits.iter().flat_map(|commit| &commit.pack.ops) {
        let op_id = op.compute_id();
        if !op_ids.insert(op_id) {
            return Err(invalid(format!("contains operation {op_id} more than once")));
        }
    }

    let edit_clock = commits.last().map_or(0, |commit| commit.edit_clock);

    clock::witness(git_repo, BUGS_CREATE, create_clock)?;
    clock::witness(git_repo, BUGS_EDIT, edit_clock)?;

    Ok(Bug {
        id: *bug_id,
        packs: commits.into_iter().map(|commit| commit.pack).collect(),
        create_clock,
        edit_clock,
    })
}

struct BugCommit {
    parents: Vec<ObjectId>,
    pack: OperationPack,
    pack_id: String,
    create_clock: Option<u64>,
    edit_clock: u64,
}

fn read_dag(git_repo: &Repository, bug_id: &BugId, tip: ObjectId) -> Result<HashMap<ObjectId, BugCommit>, Error> {
    let mut commits = HashMap::new();
    let mut pending = vec![tip];

    while let Some(commit_id) = pending.pop() {
        if commits.contains_key(&commit_id) {
            continue;
        }

        let commit = read_commit(git_repo, bug_id, commit_id)?;
        pending.extend(commit.parents.iter().copied());
        commits.insert(commit_id, commit);
    }

    Ok(commits)
}

fn read_commit(git_repo: &Repository, bug_id: &BugId, commit_id: ObjectId) -> Result<BugCommit, Error> {
    let invalid = |reason: String| Error::InvalidBug { id: *bug_id, reason };

    let commit = git_repo.find_object(commit_id).map_err(Error::git)?.try_into_commit().map_err(Error::git)?;
    let commit_ref = commit.decode().map_err(Error::git)?;
    let parents = commit_ref.parents().collect();

    let tree_id = commit_ref.tree();
    let tree = git_repo.find_object(tree_id).map_err(Error::git)?.try_into_tree().map_err(Error::git)?;
    let tree_ref = tree.decode().map_err(Error::git)?;

    let mut ops_data: Option<Vec<u8>> = None;
    let mut version: Option<u32> = None;
    let mut create_clock: Option<u64> = None;
    let mut edit_clock: Option<u64> = None;

    for entry in tree_ref.entries {
        let name = entry.filename.to_str_lossy();
        let invalid_entry = |_| invalid(format!("invalid tree entry {name} in commit {commit_id}"));

        if name == "ops" {
            let blob = git_repo
                .find_object(entry.oid.to_owned())
                .map_err(Error::git)?
                .try_into_blob()
                .map_err(Error::git)?;
            ops_data = Some(blob.data.clone());
        } else if let Some(rest) = name.strip_prefix("version-") {
            version = Some(rest.parse().map_err(invalid_entry)?);
        } else if let Some(rest) = name.strip_prefix("create-clock-") {
            create_clock = Some(rest.parse().map_err(invalid_entry)?);
        } else if let Some(rest) = name.strip_prefix("edit-clock-") {
            edit_clock = Some(rest.parse().map_err(invalid_entry)?);
        }
    }

    let version = version.ok_or_else(|| invalid(format!("version entry missing from commit {commit_id}")))?;

    if version != FORMAT_VERSION {
        return Err(invalid(format!("commit {commit_id} has unsupported format version {version}")));
    }

    let edit_clock = edit_clock.ok_or_else(|| invalid(format!("edit-clock entry missing from commit {commit_id}")))?;

    let ops_bytes = ops_data.ok_or_else(|| invalid(format!("ops blob missing from commit {commit_id}")))?;
    let pack: OperationPack = serde_json::from_slice(&ops_bytes).map_err(|err| invalid(format!("commit {commit_id} has an invalid ops blob: {err}")))?;

    Ok(BugCommit {
        parents,
        pack,
        pack_id: hex::encode(Sha256::digest(&ops_bytes)),
        create_clock,
        edit_clock,
    })
}

/// List the hex IDs of all bugs stored in the repository
pub fn list_bug_ids(git_repo: &Repository) -> Result<Vec<BugId>, Error> {
    let platform = git_repo.references().map_err(Error::git)?;
    let iter = platform.prefixed(BUGS_REF_PREFIX).map_err(Error::git)?;

    let mut ids = Vec::new();

    for reference in iter {
        let reference = reference.map_err(Error::Git)?;
        let full_name = reference.name().as_bstr().to_str_lossy().to_string();

        if let Some(id) = full_name.strip_prefix(BUGS_REF_PREFIX) {
            ids.push(id.parse().map_err(|_| Error::InvalidBugRef(full_name.clone()))?);
        }
    }

    Ok(ids)
}

/// Delete a bug by removing its ref. The underlying git objects are not purged.
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`gix::Repository`]
/// - `bug_id`: ID of the bug to delete
pub fn delete_bug(git_repo: &Repository, bug_id: &BugId) -> Result<(), Error> {
    let ref_name = format!("{BUGS_REF_PREFIX}{bug_id}");

    let reference = git_repo
        .try_find_reference(ref_name.as_str())
        .map_err(Error::git)?
        .ok_or(Error::BugNotFound(*bug_id))?;

    reference.delete().map_err(Error::git)?;
    Ok(())
}

#[cfg(test)]
mod tests;
