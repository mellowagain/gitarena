use crate::author::Author;
use crate::clock::{self, BUGS_CREATE, BUGS_EDIT};
use crate::operation::{BugStatus, Operation, OperationPack, OperationType, random_nonce};
use anyhow::{Context, Result, anyhow, ensure};
use chrono::Utc;
use gix::actor::Signature;
use gix::bstr::ByteSlice;
use gix::date::parse::TimeBuf;
use gix::objs::Tree;
use gix::objs::tree::{Entry, EntryKind};
use gix::{ObjectId, Repository};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use tracing::instrument;

/// Ref location where bugs are stored.
/// Build a path to a bug by concatenating this variable and the bug id
pub const BUGS_REF_PREFIX: &str = "refs/bugs/";

/// Sha1 of an empty file used for all clock and version tree entries
pub const EMPTY_BLOB_SHA1: &str = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";

/// Bug format version written to and accepted from the `version-<N>` tree entry
pub const FORMAT_VERSION: u32 = 4;

/// Largest edit-clock increase allowed between a non-merge commit and its parent (spec dag-entity.md §4)
pub const MAX_CLOCK_JUMP: u64 = 1_000_000;

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
        let mut created = false;

        for pack in &self.packs {
            for op in &pack.ops {
                match op.op_type {
                    OperationType::CREATE if !created => {
                        created = true;
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

                        labels.sort();
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
                    _ => {}
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
    // a clock-spammed bug elsewhere in the repo must not push this bug past the jump limit
    let edit_clock = clock::increment(git_repo, BUGS_EDIT)?.min(bug.edit_clock + MAX_CLOCK_JUMP);
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

/// Load a bug from the repository by its hex ID, rejecting it if it violates the `git-bug` spec.
/// Clock values of the bug are witnessed in the repository's Lamport clocks.
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`Repository`]
/// - `bug_id_hex`: sha256 ID of the bug to load
pub fn load_bug(git_repo: &Repository, bug_id_hex: &str) -> Result<Bug> {
    ensure!(
        bug_id_hex.len() == 64 && bug_id_hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
        "invalid bug id: {bug_id_hex}"
    );

    let ref_name = format!("{BUGS_REF_PREFIX}{bug_id_hex}");
    let tip = find_ref_target(git_repo, &ref_name)?;

    let commits = read_dag(git_repo, tip)?;
    let mut root = None;

    for (id, commit) in &commits {
        if commit.parents.is_empty() {
            ensure!(root.replace(commit).is_none(), "bug {bug_id_hex} has more than one root commit");
            continue;
        }

        let is_merge = commit.parents.len() > 1;
        ensure!(
            !is_merge || commit.pack.ops.is_empty(),
            "merge commit {id} of bug {bug_id_hex} carries operations"
        );

        for parent in &commit.parents {
            let parent_clock = commits[parent].edit_clock;

            ensure!(
                commit.edit_clock > parent_clock,
                "commit {id} of bug {bug_id_hex} does not advance the edit-clock"
            );
            ensure!(
                is_merge || commit.edit_clock - parent_clock <= MAX_CLOCK_JUMP,
                "commit {id} of bug {bug_id_hex} jumps the edit-clock by more than {MAX_CLOCK_JUMP}"
            );
        }
    }

    let root = root.with_context(|| format!("bug {bug_id_hex} has no root commit"))?;

    let create_clock = root
        .create_clock
        .filter(|clock| *clock > 0)
        .with_context(|| format!("root commit of bug {bug_id_hex} has no create-clock"))?;

    let create_op = root
        .pack
        .ops
        .first()
        .with_context(|| format!("root commit of bug {bug_id_hex} has no operations"))?;
    ensure!(
        create_op.compute_id()? == bug_id_hex,
        "bug {bug_id_hex} does not match the ID of its first operation"
    );

    let mut commits: Vec<BugCommit> = commits.into_values().collect();
    commits.sort_by(|a, b| a.edit_clock.cmp(&b.edit_clock).then_with(|| a.pack_id.cmp(&b.pack_id)));

    let mut op_ids = HashSet::new();

    for op in commits.iter().flat_map(|commit| &commit.pack.ops) {
        let op_id = op.compute_id()?;
        ensure!(op_ids.insert(op_id.clone()), "bug {bug_id_hex} contains operation {op_id} more than once");
    }

    let edit_clock = commits.last().map_or(0, |commit| commit.edit_clock);

    clock::witness(git_repo, BUGS_CREATE, create_clock)?;
    clock::witness(git_repo, BUGS_EDIT, edit_clock)?;

    Ok(Bug {
        id: bug_id_hex.to_owned(),
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

fn read_dag(git_repo: &Repository, tip: ObjectId) -> Result<HashMap<ObjectId, BugCommit>> {
    let mut commits = HashMap::new();
    let mut pending = vec![tip];

    while let Some(commit_id) = pending.pop() {
        if commits.contains_key(&commit_id) {
            continue;
        }

        let commit = read_commit(git_repo, commit_id)?;
        pending.extend(commit.parents.iter().copied());
        commits.insert(commit_id, commit);
    }

    Ok(commits)
}

fn read_commit(git_repo: &Repository, commit_id: ObjectId) -> Result<BugCommit> {
    let commit = git_repo.find_object(commit_id)?.try_into_commit()?;
    let commit_ref = commit.decode()?;
    let parents = commit_ref.parents().collect();

    let tree_id = commit_ref.tree();
    let tree = git_repo.find_object(tree_id)?.try_into_tree()?;
    let tree_ref = tree.decode()?;

    let mut ops_data: Option<Vec<u8>> = None;
    let mut version: Option<u32> = None;
    let mut create_clock: Option<u64> = None;
    let mut edit_clock: Option<u64> = None;

    for entry in tree_ref.entries {
        let name = entry.filename.to_str_lossy();
        let invalid_entry = || format!("invalid tree entry {name} in bug commit {commit_id}");

        if name == "ops" {
            let blob = git_repo.find_object(entry.oid.to_owned())?.try_into_blob()?;
            ops_data = Some(blob.data.clone());
        } else if let Some(rest) = name.strip_prefix("version-") {
            version = Some(rest.parse().with_context(invalid_entry)?);
        } else if let Some(rest) = name.strip_prefix("create-clock-") {
            create_clock = Some(rest.parse().with_context(invalid_entry)?);
        } else if let Some(rest) = name.strip_prefix("edit-clock-") {
            edit_clock = Some(rest.parse().with_context(invalid_entry)?);
        }
    }

    let version = version.with_context(|| format!("version entry missing from bug commit {commit_id}"))?;
    ensure!(version == FORMAT_VERSION, "bug commit {commit_id} has unsupported format version {version}");

    let edit_clock = edit_clock.with_context(|| format!("edit-clock entry missing from bug commit {commit_id}"))?;

    let ops_bytes = ops_data.context("ops blob missing from bug commit tree")?;
    let pack: OperationPack = serde_json::from_slice(&ops_bytes).context("failed to deserialize operation pack")?;

    Ok(BugCommit {
        parents,
        pack,
        pack_id: hex::encode(Sha256::digest(&ops_bytes)),
        create_clock,
        edit_clock,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::set_title;
    use gix::refs::transaction::PreviousValue;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;
    use uuid::Uuid;

    const CREATE_OP: &str = r#"{"type":1,"timestamp":1,"nonce":"n1","title":"title","message":"body","files":null}"#;
    const ROOT: &[&str] = &["create-clock-1", "edit-clock-1", "version-4"];

    type InvalidCase = (&'static str, fn(&Repository));

    fn bug_id() -> String {
        hex::encode(Sha256::digest(CREATE_OP))
    }

    fn pack(ops: &[&str]) -> String {
        format!(r#"{{"author":{{"id":"a"}},"ops":[{}]}}"#, ops.join(","))
    }

    fn comment(message: &str) -> String {
        format!(r#"{{"type":3,"timestamp":2,"nonce":"{message}","message":"{message}","files":null}}"#)
    }

    /// Write a commit with `ops` plus an empty blob named after each of `entries` and point the bug ref at it
    fn commit(repo: &Repository, ops: &str, entries: &[&str], parents: &[ObjectId]) -> ObjectId {
        let ops_id = repo.write_blob(ops).expect("ops blob").detach();
        let empty_id = repo.write_blob(b"").expect("empty blob").detach();

        let mut entries: Vec<Entry> = entries
            .iter()
            .map(|name| Entry {
                mode: EntryKind::Blob.into(),
                filename: (*name).into(),
                oid: empty_id,
            })
            .collect();

        entries.push(Entry {
            mode: EntryKind::Blob.into(),
            filename: "ops".into(),
            oid: ops_id,
        });
        entries.sort_by(|a, b| a.filename.cmp(&b.filename));

        let tree_id = repo.write_object(&Tree { entries }).expect("tree").detach();

        let sig = Signature {
            name: "test".into(),
            email: "test@gitarena.local".into(),
            time: gix::date::Time::default(),
        };

        let mut time_buffer_committer = TimeBuf::default();
        let mut time_buffer_author = TimeBuf::default();

        let commit_id = repo
            .new_commit_as(
                sig.to_ref(&mut time_buffer_committer),
                sig.to_ref(&mut time_buffer_author),
                "",
                tree_id,
                parents.iter().copied(),
            )
            .expect("commit")
            .id;

        repo.reference(format!("{BUGS_REF_PREFIX}{}", bug_id()), commit_id, PreviousValue::Any, "test")
            .expect("bug ref");

        commit_id
    }

    #[test]
    fn concurrent_packs_are_ordered_by_edit_clock_then_pack_id() {
        let dir = tempdir().expect("tempdir");
        let repo = gix::init(dir.path()).expect("gix::init");

        let pack_a = pack(&[comment("a").as_str()]);
        let pack_b = pack(&[comment("b").as_str()]);

        let root = commit(&repo, &pack(&[CREATE_OP]), ROOT, &[]);
        let a = commit(&repo, &pack_a, &["edit-clock-2", "version-4"], &[root]);
        let b = commit(&repo, &pack_b, &["edit-clock-2", "version-4"], &[root]);
        commit(&repo, &pack(&[]), &["edit-clock-3", "version-4"], &[b, a]);

        let mut expected = vec![(hex::encode(Sha256::digest(&pack_a)), "a"), (hex::encode(Sha256::digest(&pack_b)), "b")];
        expected.sort();
        let expected: Vec<&str> = expected.into_iter().map(|(_, body)| body).collect();

        let bug = load_bug(&repo, &bug_id()).expect("load_bug");
        let bodies: Vec<String> = bug.snapshot().comments.into_iter().map(|comment| comment.body).collect();

        assert_eq!(bodies, expected);
        assert_eq!(bug.packs.len(), 4);
        assert_eq!((bug.create_clock, bug.edit_clock), (1, 3));
        assert_eq!(clock::read(&repo, BUGS_CREATE).expect("read"), 1);
        assert_eq!(clock::read(&repo, BUGS_EDIT).expect("read"), 3);
    }

    #[test]
    fn invalid_bugs_are_rejected() {
        let cases: [InvalidCase; 9] = [
            ("unsupported format version 5", |repo| {
                commit(repo, &pack(&[CREATE_OP]), &["create-clock-1", "edit-clock-1", "version-5"], &[]);
            }),
            ("edit-clock entry missing", |repo| {
                commit(repo, &pack(&[CREATE_OP]), &["create-clock-1", "version-4"], &[]);
            }),
            ("has no create-clock", |repo| {
                commit(repo, &pack(&[CREATE_OP]), &["edit-clock-1", "version-4"], &[]);
            }),
            ("does not match the ID of its first operation", |repo| {
                commit(repo, &pack(&[comment("a").as_str()]), ROOT, &[]);
            }),
            ("does not advance the edit-clock", |repo| {
                let root = commit(repo, &pack(&[CREATE_OP]), ROOT, &[]);
                commit(repo, &pack(&[comment("a").as_str()]), &["edit-clock-1", "version-4"], &[root]);
            }),
            ("jumps the edit-clock", |repo| {
                let root = commit(repo, &pack(&[CREATE_OP]), ROOT, &[]);
                commit(repo, &pack(&[comment("a").as_str()]), &["edit-clock-1000002", "version-4"], &[root]);
            }),
            ("carries operations", |repo| {
                let root = commit(repo, &pack(&[CREATE_OP]), ROOT, &[]);
                let a = commit(repo, &pack(&[comment("a").as_str()]), &["edit-clock-2", "version-4"], &[root]);
                let b = commit(repo, &pack(&[comment("b").as_str()]), &["edit-clock-2", "version-4"], &[root]);
                commit(repo, &pack(&[comment("c").as_str()]), &["edit-clock-3", "version-4"], &[a, b]);
            }),
            ("more than one root", |repo| {
                let root = commit(repo, &pack(&[CREATE_OP]), ROOT, &[]);
                let other = commit(repo, &pack(&[comment("a").as_str()]), ROOT, &[]);
                commit(repo, &pack(&[]), &["edit-clock-2", "version-4"], &[root, other]);
            }),
            ("more than once", |repo| {
                let root = commit(repo, &pack(&[CREATE_OP]), ROOT, &[]);
                commit(repo, &pack(&[CREATE_OP]), &["edit-clock-2", "version-4"], &[root]);
            }),
        ];

        for (expected, build) in cases {
            let dir = tempdir().expect("tempdir");
            let repo = gix::init(dir.path()).expect("gix::init");
            build(&repo);

            let err = load_bug(&repo, &bug_id()).expect_err(expected);
            assert!(format!("{err:#}").contains(expected), "expected error containing {expected:?}, got {err:#}");
        }
    }

    #[test]
    fn writes_use_repository_clocks() {
        let dir = tempdir().expect("tempdir");
        let repo = gix::init(dir.path()).expect("gix::init");
        let author = Author::from_user(Uuid::nil(), "testuser", "testuser@gitarena.local", 1_700_000_000);

        let first = create_bug(&repo, author.clone(), "first".into(), "body".into()).expect("create_bug");
        let second = create_bug(&repo, author.clone(), "second".into(), "body".into()).expect("create_bug");
        assert_eq!((second.create_clock, second.edit_clock), (2, 2));

        let first = set_title(&repo, first, author, "renamed".into()).expect("set_title");
        assert_eq!(first.edit_clock, 3);

        let loaded = load_bug(&repo, &first.id).expect("load_bug");
        assert_eq!(loaded.packs[1].ops[0].was.as_deref(), Some("first"));
        assert_eq!(loaded.snapshot().title, "renamed");
    }

    #[test]
    fn labels_are_sorted_and_later_creates_are_ignored() {
        let ops = pack(&[
            CREATE_OP,
            r#"{"type":5,"timestamp":2,"nonce":"n2","added":["b","a"],"removed":[]}"#,
            r#"{"type":1,"timestamp":3,"nonce":"n3","title":"other","message":"other","files":null}"#,
        ]);
        let pack: OperationPack = serde_json::from_str(&ops).expect("pack deserializes");

        let bug = Bug {
            id: bug_id(),
            packs: vec![pack],
            create_clock: 1,
            edit_clock: 1,
        };

        let snapshot = bug.snapshot();
        assert_eq!(snapshot.labels, ["a", "b"]);
        assert_eq!(snapshot.title, "title");
        assert_eq!(snapshot.body, "body");
    }

    #[test]
    fn unknown_operation_types_are_skipped() {
        let ops = r#"{"author":{"id":"a"},"ops":[{"type":1,"timestamp":1,"nonce":"n1","title":"title","message":"body","files":null},{"type":7,"timestamp":2,"nonce":"n2","metadata":{"k":"v"}},{"type":300,"timestamp":3,"nonce":"n3","status":"not-a-number","added":{}}]}"#;
        let pack: OperationPack = serde_json::from_str(ops).expect("pack with unknown ops deserializes");

        let create_raw = r#"{"type":1,"timestamp":1,"nonce":"n1","title":"title","message":"body","files":null}"#;
        let bug_id = hex::encode(Sha256::digest(create_raw.as_bytes()));
        assert_eq!(pack.ops[0].compute_id().expect("compute_id"), bug_id);

        let bug = Bug {
            id: bug_id,
            packs: vec![pack],
            create_clock: 1,
            edit_clock: 1,
        };

        let snapshot = bug.snapshot();
        assert_eq!(snapshot.title, "title");
        assert_eq!(snapshot.body, "body");
    }
}
