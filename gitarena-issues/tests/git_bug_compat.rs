use gitarena_issues::author::Author;
use gitarena_issues::bug::create_bug;
use gitarena_issues::ops::{add_comment, set_status};
use serde_json::Value;
use std::path::Path;
use std::process::{Command, exit};
use tempfile::tempdir;
use uuid::Uuid;
use which::which;

#[test]
fn git_bug_can_read_created_bug() {
    if which("git-bug").is_err() {
        eprintln!("git-bug not installed, skipping tests...");
        return;
    }

    let dir = tempdir().expect("tempdir");
    let repo = gix::init(dir.path()).expect("gix::init");

    let author = make_author();
    let bug = create_bug(&repo, author.clone(), "Test issue title".into(), "Body of the issue.".into()).expect("create_bug");

    let bug_id = bug.id.clone();

    let list_out = Command::new("git-bug").args(["bug"]).current_dir(dir.path()).output().expect("git-bug bug");

    assert!(list_out.status.success(), "git-bug bug failed: {}", String::from_utf8_lossy(&list_out.stderr));

    let list_text = String::from_utf8_lossy(&list_out.stdout);

    assert!(
        list_text.contains("Test issue title"),
        "git-bug bug did not list our issue.\nOutput:\n{list_text}"
    );

    let show_out = Command::new("git-bug")
        .args(["bug", "show", "--format", "json", &bug_id])
        .current_dir(dir.path())
        .output()
        .expect("git-bug bug show");

    assert!(
        show_out.status.success(),
        "git-bug bug show failed: {}",
        String::from_utf8_lossy(&show_out.stderr)
    );

    let show_text = String::from_utf8_lossy(&show_out.stdout);
    let json: Value = serde_json::from_str(&show_text).unwrap_or_else(|err| panic!("git-bug show output is not valid JSON: {err}\nOutput:\n{show_text}"));

    assert_eq!(json["title"].as_str().unwrap_or(""), "Test issue title", "title mismatch");
    assert_eq!(json["status"].as_str().unwrap_or(""), "open", "status should be open");
    assert_eq!(json["id"].as_str().unwrap_or(""), bug_id, "id mismatch");

    let comments = json["comments"].as_array().expect("comments array");
    assert!(!comments.is_empty(), "expected at least one comment (the body)");
    assert_eq!(comments[0]["message"].as_str().unwrap_or(""), "Body of the issue.", "body mismatch");
}

#[test]
fn git_bug_can_read_comment_and_closed_status() {
    if which("git-bug").is_err() {
        eprintln!("git-bug not installed, skipping tests...");
        return;
    }

    let dir = tempdir().expect("tempdir");
    let repo = gix::init(dir.path()).expect("gix::init");

    let author = make_author();
    let bug = create_bug(&repo, author.clone(), "Issue with comment".into(), "Initial body.".into()).expect("create_bug");

    let bug_id = bug.id.clone();

    let (bug, _) = add_comment(&repo, bug, author.clone(), "A follow-up comment.".into()).expect("add_comment");

    let _ = set_status(&repo, bug, author, false).expect("set_status closed");

    let show_out = Command::new("git-bug")
        .args(["bug", "show", "--format", "json", &bug_id])
        .current_dir(dir.path())
        .output()
        .expect("git-bug bug show");

    assert!(
        show_out.status.success(),
        "git-bug bug show failed: {}",
        String::from_utf8_lossy(&show_out.stderr)
    );

    let show_text = String::from_utf8_lossy(&show_out.stdout);
    let json: Value = serde_json::from_str(&show_text).unwrap_or_else(|err| panic!("not valid JSON: {err}\nOutput:\n{show_text}"));

    assert_eq!(json["status"].as_str().unwrap_or(""), "closed", "status should be closed");

    let comments = json["comments"].as_array().expect("comments array");
    assert_eq!(comments.len(), 2, "expected 2 comments (body + follow-up)");
    assert_eq!(comments[1]["message"].as_str().unwrap_or(""), "A follow-up comment.", "comment body mismatch");
}

fn make_author() -> Author {
    Author::from_user(Uuid::nil(), "testuser", "testuser@gitarena.local", 1_700_000_000)
}
