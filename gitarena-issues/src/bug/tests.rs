use super::*;
use crate::ops::set_title;
use gix::refs::transaction::PreviousValue;
use pretty_assertions::assert_eq;
use tempfile::tempdir;
use uuid::Uuid;

const CREATE_OP: &str = r#"{"type":1,"timestamp":1,"nonce":"n1","title":"title","message":"body","files":null}"#;
const ROOT: &[&str] = &["create-clock-1", "edit-clock-1", "version-4"];

type InvalidCase = (&'static str, fn(&Repository) -> Result<ObjectId>);

fn bug_id() -> String {
    hex::encode(Sha256::digest(CREATE_OP))
}

fn pack(ops: &[&str]) -> String {
    format!(r#"{{"author":{{"id":"a"}},"ops":[{}]}}"#, ops.join(","))
}

fn comment(message: &str) -> String {
    format!(r#"{{"type":3,"timestamp":2,"nonce":"{message}","message":"{message}","files":null}}"#)
}

fn commit(repo: &Repository, ops: &str, entries: &[&str], parents: &[ObjectId]) -> Result<ObjectId> {
    let ops_id = repo.write_blob(ops)?.detach();
    let empty_id = repo.write_blob(b"")?.detach();

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

    let tree_id = repo.write_object(&Tree { entries })?.detach();

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
        )?
        .id;

    repo.reference(format!("{BUGS_REF_PREFIX}{}", bug_id()), commit_id, PreviousValue::Any, "test")?;

    Ok(commit_id)
}

#[test]
fn concurrent_packs_are_ordered_by_edit_clock_then_pack_id() -> Result<()> {
    let dir = tempdir()?;
    let repo = gix::init(dir.path())?;

    let pack_a = pack(&[comment("a").as_str()]);
    let pack_b = pack(&[comment("b").as_str()]);

    let root = commit(&repo, &pack(&[CREATE_OP]), ROOT, &[])?;
    let a = commit(&repo, &pack_a, &["edit-clock-2", "version-4"], &[root])?;
    let b = commit(&repo, &pack_b, &["edit-clock-2", "version-4"], &[root])?;
    commit(&repo, &pack(&[]), &["edit-clock-3", "version-4"], &[b, a])?;

    let mut expected = vec![(hex::encode(Sha256::digest(&pack_a)), "a"), (hex::encode(Sha256::digest(&pack_b)), "b")];
    expected.sort();
    let expected: Vec<&str> = expected.into_iter().map(|(_, body)| body).collect();

    let bug = load_bug(&repo, &bug_id())?;
    let bodies: Vec<String> = bug.snapshot().comments.into_iter().map(|comment| comment.body).collect();

    assert_eq!(bodies, expected);
    assert_eq!(bug.packs.len(), 4);
    assert_eq!((bug.create_clock, bug.edit_clock), (1, 3));
    assert_eq!(clock::read(&repo, BUGS_CREATE)?, 1);
    assert_eq!(clock::read(&repo, BUGS_EDIT)?, 3);

    Ok(())
}

#[test]
fn invalid_bugs_are_rejected() -> Result<()> {
    let cases: [InvalidCase; 9] = [
        ("unsupported format version 5", |repo| {
            commit(repo, &pack(&[CREATE_OP]), &["create-clock-1", "edit-clock-1", "version-5"], &[])
        }),
        ("edit-clock entry missing", |repo| {
            commit(repo, &pack(&[CREATE_OP]), &["create-clock-1", "version-4"], &[])
        }),
        ("has no create-clock", |repo| {
            commit(repo, &pack(&[CREATE_OP]), &["edit-clock-1", "version-4"], &[])
        }),
        ("does not match the ID of its first operation", |repo| {
            commit(repo, &pack(&[comment("a").as_str()]), ROOT, &[])
        }),
        ("does not advance the edit-clock", |repo| {
            let root = commit(repo, &pack(&[CREATE_OP]), ROOT, &[])?;
            commit(repo, &pack(&[comment("a").as_str()]), &["edit-clock-1", "version-4"], &[root])
        }),
        ("jumps the edit-clock", |repo| {
            let root = commit(repo, &pack(&[CREATE_OP]), ROOT, &[])?;
            commit(repo, &pack(&[comment("a").as_str()]), &["edit-clock-1000002", "version-4"], &[root])
        }),
        ("carries operations", |repo| {
            let root = commit(repo, &pack(&[CREATE_OP]), ROOT, &[])?;
            let a = commit(repo, &pack(&[comment("a").as_str()]), &["edit-clock-2", "version-4"], &[root])?;
            let b = commit(repo, &pack(&[comment("b").as_str()]), &["edit-clock-2", "version-4"], &[root])?;
            commit(repo, &pack(&[comment("c").as_str()]), &["edit-clock-3", "version-4"], &[a, b])
        }),
        ("more than one root", |repo| {
            let root = commit(repo, &pack(&[CREATE_OP]), ROOT, &[])?;
            let other = commit(repo, &pack(&[comment("a").as_str()]), ROOT, &[])?;
            commit(repo, &pack(&[]), &["edit-clock-2", "version-4"], &[root, other])
        }),
        ("more than once", |repo| {
            let root = commit(repo, &pack(&[CREATE_OP]), ROOT, &[])?;
            commit(repo, &pack(&[CREATE_OP]), &["edit-clock-2", "version-4"], &[root])
        }),
    ];

    for (expected, build) in cases {
        let dir = tempdir()?;
        let repo = gix::init(dir.path())?;
        build(&repo)?;

        let err = load_bug(&repo, &bug_id()).expect_err(expected);
        assert!(format!("{err:#}").contains(expected), "expected error containing {expected:?}, got {err:#}");
    }

    Ok(())
}

#[test]
fn writes_use_repository_clocks() -> Result<()> {
    let dir = tempdir()?;
    let repo = gix::init(dir.path())?;
    let author = Author::from_user(Uuid::nil(), "testuser", "testuser@gitarena.local", 1_700_000_000);

    let first = create_bug(&repo, author.clone(), "first".into(), "body".into())?;
    let second = create_bug(&repo, author.clone(), "second".into(), "body".into())?;
    assert_eq!((second.create_clock, second.edit_clock), (2, 2));

    let first = set_title(&repo, first, author, "renamed".into())?;
    assert_eq!(first.edit_clock, 3);

    let loaded = load_bug(&repo, &first.id)?;
    assert_eq!(loaded.packs[1].ops[0].was.as_deref(), Some("first"));
    assert_eq!(loaded.snapshot().title, "renamed");

    Ok(())
}

#[test]
fn labels_are_sorted_and_later_creates_are_ignored() -> Result<()> {
    let ops = pack(&[
        CREATE_OP,
        r#"{"type":5,"timestamp":2,"nonce":"n2","added":["b","a"],"removed":[]}"#,
        r#"{"type":1,"timestamp":3,"nonce":"n3","title":"other","message":"other","files":null}"#,
    ]);
    let pack: OperationPack = serde_json::from_str(&ops)?;

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

    Ok(())
}

#[test]
fn unknown_operation_types_are_skipped() -> Result<()> {
    let ops = r#"{"author":{"id":"a"},"ops":[{"type":1,"timestamp":1,"nonce":"n1","title":"title","message":"body","files":null},{"type":7,"timestamp":2,"nonce":"n2","metadata":{"k":"v"}},{"type":300,"timestamp":3,"nonce":"n3","status":"not-a-number","added":{}}]}"#;
    let pack: OperationPack = serde_json::from_str(ops)?;

    let create_raw = r#"{"type":1,"timestamp":1,"nonce":"n1","title":"title","message":"body","files":null}"#;
    let bug_id = hex::encode(Sha256::digest(create_raw.as_bytes()));
    assert_eq!(pack.ops[0].compute_id()?, bug_id);

    let bug = Bug {
        id: bug_id,
        packs: vec![pack],
        create_clock: 1,
        edit_clock: 1,
    };

    let snapshot = bug.snapshot();
    assert_eq!(snapshot.title, "title");
    assert_eq!(snapshot.body, "body");

    Ok(())
}
