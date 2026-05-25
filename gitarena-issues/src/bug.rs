use crate::author::Author;
use crate::operation::{BugStatus, Operation, OperationPack, OperationType, random_nonce};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use gix::actor::Signature;
use gix::bstr::ByteSlice;
use gix::date::parse::TimeBuf;
use gix::objs::Tree;
use gix::objs::tree::{Entry, EntryKind};
use gix::{ObjectId, Repository};
use tracing::instrument;

/// Ref location where bugs are stored.
/// Build a path to a bug by concatenating this variable and the bug id
pub const BUGS_REF_PREFIX: &str = "refs/bugs/";

/// Sha1 of an empty file used for all clock and version tree entries
pub const EMPTY_BLOB_SHA1: &str = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";

/// In-memory representation of a `git-bug` bug with all its operation packs loaded.
/// Load from a repository with [`load_bug`] or create with [`create_bug`]
#[derive(Debug, Clone)]
pub struct Bug {
    /// sha256 ID of the bug, derived from its initial CREATE operation
    pub id: String,
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

        for pack in &self.packs {
            for op in &pack.ops {
                match op.op_type {
                    OperationType::CREATE => {
                        title = op.title.as_deref().unwrap_or_default().to_owned();
                        body = op.message.as_deref().unwrap_or_default().to_owned();
                        created_at = op.timestamp;
                    }
                    OperationType::SET_TITLE => {
                        if let Some(t) = &op.title {
                            title.clone_from(t);
                        }
                    }
                    OperationType::ADD_COMMENT => {
                        let op_id = op.compute_id().unwrap_or_default();

                        comments.push(CommentSnapshot {
                            id: op_id,
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
                    }
                    OperationType::EDIT_COMMENT => {
                        if let (Some(target), Some(new_body)) = (&op.target, &op.message) {
                            if *target == self.id {
                                body.clone_from(new_body);
                            } else if let Some(comment) = comments.iter_mut().find(|c| c.id == *target) {
                                comment.body.clone_from(new_body);
                                comment.edited_at = Some(op.timestamp);
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }

        BugSnapshot {
            id: self.id.clone(),
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
pub struct BugSnapshot {
    /// sha256 ID of the bug
    pub id: String,
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
pub struct CommentSnapshot {
    /// sha256 of the `ADD_COMMENT` operation json, used as the operation ID in `git-bug`
    pub id: String,
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
#[instrument(err, skip(git_repo))]
pub fn create_bug(git_repo: &Repository, author: Author, title: String, body: String) -> Result<Bug> {
    let now = Utc::now().timestamp();
    let create_clock = u64::try_from(now).unwrap_or(0);
    let edit_clock = 1_u64;

    let create_op = Operation {
        op_type: OperationType::CREATE,
        timestamp: now,
        nonce: random_nonce(),
        title: Some(title),
        message: Some(body),
        status: None,
        added: None,
        removed: None,
        target: None,
        files: Some(serde_json::Value::Null),
    };

    let bug_id = create_op.compute_id()?;

    let pack = OperationPack {
        author: author.clone(),
        ops: vec![create_op],
    };

    author.write_identity_if_absent(git_repo)?;
    write_pack_as_commit(git_repo, &pack, create_clock, edit_clock, None, &bug_id)?;

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
#[instrument(err, skip(git_repo))]
pub fn append_operation(git_repo: &Repository, mut bug: Bug, author: Author, op: Operation) -> Result<Bug> {
    let edit_clock = bug.edit_clock + 1;
    let ref_name = format!("{BUGS_REF_PREFIX}{}", bug.id);
    let current_tip = find_ref_target(git_repo, &ref_name)?;

    author.write_identity_if_absent(git_repo)?;

    let pack = OperationPack { author, ops: vec![op] };
    write_pack_as_commit(git_repo, &pack, 0, edit_clock, Some(current_tip), &bug.id)?;

    bug.packs.push(pack);
    bug.edit_clock = edit_clock;

    Ok(bug)
}

fn write_pack_as_commit(
    git_repo: &Repository,
    pack: &OperationPack,
    create_clock: u64,
    edit_clock: u64,
    parent: Option<ObjectId>,
    bug_id_hex: &str,
) -> Result<ObjectId> {
    let ops_json = serde_json::to_vec(pack).context("failed to serialize operation pack")?;

    let ops_blob_id = git_repo
        .write_object(gix::objs::BlobRef { data: &ops_json })
        .context("failed to write ops blob")?
        .detach();

    let empty_blob_id = ObjectId::from_hex(EMPTY_BLOB_SHA1.as_bytes()).context("invalid empty blob sha")?;

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
            filename: "version-4".into(),
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
    let tree_id = git_repo.write_object(&tree).context("failed to write tree")?.detach();

    let now = Utc::now();

    let sig = Signature {
        name: pack.author.name.as_bytes().into(),
        email: pack.author.email.as_bytes().into(),
        time: gix::date::Time {
            seconds: now.timestamp(),
            offset: 0,
        },
    };

    let parents: Vec<ObjectId> = parent.into_iter().collect();
    let ref_name = format!("{BUGS_REF_PREFIX}{bug_id_hex}");

    let mut time_buffer_committer = TimeBuf::default();
    let mut time_buffer_author = TimeBuf::default();

    let commit_id = git_repo
        .commit_as(
            sig.to_ref(&mut time_buffer_committer),
            sig.to_ref(&mut time_buffer_author),
            ref_name.as_str(),
            "",
            tree_id,
            parents,
        )
        .context("failed to write bug commit")?
        .detach();

    Ok(commit_id)
}

fn find_ref_target(git_repo: &Repository, ref_name: &str) -> Result<ObjectId> {
    let reference = git_repo.find_reference(ref_name).with_context(|| format!("bug ref not found: {ref_name}"))?;
    Ok(reference.id().detach())
}

/// Load a bug from the repository by its hex ID.
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`Repository`]
/// - `bug_id_hex`: sha256 ID of the bug to load
pub fn load_bug(git_repo: &Repository, bug_id_hex: &str) -> Result<Bug> {
    let ref_name = format!("{BUGS_REF_PREFIX}{bug_id_hex}");
    let tip = find_ref_target(git_repo, &ref_name)?;

    let mut packs = Vec::new();
    let mut create_clock = 0_u64;
    let mut edit_clock = 0_u64;

    let commits = collect_commit_chain(git_repo, tip)?;

    for commit_id in commits.iter().rev() {
        let (pack, cc, ec) = read_pack_from_commit(git_repo, *commit_id)?;

        if cc > 0 {
            create_clock = cc;
        }

        if ec > edit_clock {
            edit_clock = ec;
        }

        packs.push(pack);
    }

    Ok(Bug {
        id: bug_id_hex.to_owned(),
        packs,
        create_clock,
        edit_clock,
    })
}

fn collect_commit_chain(git_repo: &Repository, tip: ObjectId) -> Result<Vec<ObjectId>> {
    let mut chain = Vec::new();
    let mut current = tip;

    loop {
        chain.push(current);

        let commit = git_repo.find_object(current)?.try_into_commit()?;
        let commit_ref = commit.decode()?;

        match commit_ref.parents().next() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Ok(chain)
}

fn read_pack_from_commit(git_repo: &Repository, commit_id: ObjectId) -> Result<(OperationPack, u64, u64)> {
    let commit = git_repo.find_object(commit_id)?.try_into_commit()?;
    let commit_ref = commit.decode()?;

    let tree_id = commit_ref.tree();
    let tree = git_repo.find_object(tree_id)?.try_into_tree()?;
    let tree_ref = tree.decode()?;

    let mut ops_data: Option<Vec<u8>> = None;
    let mut create_clock = 0_u64;
    let mut edit_clock = 0_u64;

    for entry in tree_ref.entries {
        let name = entry.filename.to_str_lossy();

        if name == "ops" {
            let blob = git_repo.find_object(entry.oid.to_owned())?.try_into_blob()?;
            ops_data = Some(blob.data.clone());
        } else if let Some(rest) = name.strip_prefix("create-clock-") {
            create_clock = rest.parse().unwrap_or(0);
        } else if let Some(rest) = name.strip_prefix("edit-clock-") {
            edit_clock = rest.parse().unwrap_or(0);
        }
    }

    let ops_bytes = ops_data.context("ops blob missing from bug commit tree")?;
    let pack: OperationPack = serde_json::from_slice(&ops_bytes).context("failed to deserialize operation pack")?;

    Ok((pack, create_clock, edit_clock))
}

/// List the hex IDs of all bugs stored in the repository
pub fn list_bug_ids(git_repo: &Repository) -> Result<Vec<String>> {
    let platform = git_repo.references()?;
    let iter = platform.prefixed(BUGS_REF_PREFIX)?;

    let mut ids = Vec::new();

    for reference in iter {
        let reference = reference.map_err(|err| anyhow!("{err}"))?;
        let full_name = reference.name().as_bstr().to_str_lossy().to_string();

        if let Some(id) = full_name.strip_prefix(BUGS_REF_PREFIX) {
            ids.push(id.to_owned());
        }
    }

    Ok(ids)
}

/// Delete a bug by removing its ref. The underlying git objects are not purged.
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`gix::Repository`]
/// - `bug_id_hex`: sha256 ID of the bug to delete
pub fn delete_bug(git_repo: &Repository, bug_id_hex: &str) -> Result<()> {
    let ref_name = format!("{BUGS_REF_PREFIX}{bug_id_hex}");

    let reference = git_repo
        .find_reference(ref_name.as_str())
        .with_context(|| format!("bug ref not found: {ref_name}"))?;

    reference.delete().context("failed to delete bug ref")?;
    Ok(())
}
