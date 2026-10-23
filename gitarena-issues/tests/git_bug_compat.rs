use common::{GitBug, make_author};
use gitarena_issues::bug::{create_bug, list_bug_ids, load_bug};
use gitarena_issues::operation::BugStatus;
use gitarena_issues::ops::{add_comment, set_status};
use pretty_assertions::{assert_eq, assert_matches};

mod common;

#[test_with::executable(git-bug)]
fn git_bug_can_read_created_bug() {
    let git_bug = GitBug::new();

    let repo = &git_bug.repo;

    let author = make_author();
    let bug = create_bug(repo, author.clone(), "Test issue title".into(), "Body of the issue.".into()).expect("create_bug");

    let bug_id = bug.id.clone();

    let list_text = git_bug.run(&["bug"]);

    assert!(
        list_text.contains("Test issue title"),
        "git-bug bug did not list our issue.\nOutput:\n{list_text}"
    );

    let json = git_bug.show(&bug_id);

    assert_matches!(json["title"].as_str(), Some("Test issue title"));
    assert_matches!(json["status"].as_str(), Some("open"));
    assert_matches!(json["id"].as_str(), Some(id) if id == bug_id);

    let comments = json["comments"].as_array().expect("comments array");
    assert!(!comments.is_empty(), "expected at least one comment (the body)");
    assert_matches!(comments[0]["message"].as_str(), Some("Body of the issue."));
}

#[test_with::executable(git-bug)]
fn git_bug_can_read_comment_and_closed_status() {
    let git_bug = GitBug::new();

    let repo = &git_bug.repo;

    let author = make_author();
    let bug = create_bug(repo, author.clone(), "Issue with comment".into(), "Initial body.".into()).expect("create_bug");

    let bug_id = bug.id.clone();

    let (bug, _) = add_comment(repo, bug, author.clone(), "A follow-up comment.".into()).expect("add_comment");

    let _ = set_status(repo, bug, author, BugStatus::CLOSED).expect("set_status closed");

    let json = git_bug.show(&bug_id);

    assert_matches!(json["status"].as_str(), Some("closed"));

    let comments = json["comments"].as_array().expect("comments array");
    assert_eq!(comments.len(), 2, "expected 2 comments (body + follow-up)");
    assert_matches!(comments[1]["message"].as_str(), Some("A follow-up comment."));
}

#[test_with::executable(git-bug)]
fn comment_ids_match_git_bug_for_html_characters() {
    let git_bug = GitBug::new();

    let repo = &git_bug.repo;

    git_bug.run(&["user", "new", "--non-interactive", "-n", "tester", "-e", "tester@gitarena.local"]);
    git_bug.run(&["bug", "new", "--non-interactive", "-t", "title", "-m", "body"]);

    let bug_id = list_bug_ids(repo).expect("list_bug_ids").pop().expect("bug id");
    git_bug.run(&["bug", "comment", "new", &bug_id, "--non-interactive", "-m", "a <b>tag</b> & stuff"]);

    let show = git_bug.show(&bug_id);
    let combined_id = show["comments"][1]["id"].as_str().expect("comment id");

    let snapshot = load_bug(repo, &bug_id).expect("load_bug").snapshot();
    let comment_id = &snapshot.comments[0].id;

    // Combined IDs take the secondary (comment) ID at positions 1, 3, 5, 9 and every 5th from 14 (spec ids.md §5.1)
    let secondary: String = combined_id
        .chars()
        .enumerate()
        .filter(|(i, _)| matches!(i, 1 | 3 | 5 | 9) || (*i >= 10 && i % 5 == 4))
        .map(|(_, c)| c)
        .collect();

    assert!(
        comment_id.starts_with(&secondary),
        "comment id {comment_id} does not match git-bug combined id {combined_id}"
    );
}
