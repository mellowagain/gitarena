use anyhow::{Context, Result};
use common::{GitBug, make_author};
use gitarena_issues::bug::{MergeOutcome, create_bug, list_bug_ids, load_bug, merge_bug};
use gitarena_issues::operation::BugStatus;
use gitarena_issues::ops::{add_comment, set_status};
use pretty_assertions::{assert_eq, assert_matches};

mod common;

#[test_with::executable(git-bug)]
fn git_bug_can_read_created_bug() -> Result<()> {
    let git_bug = GitBug::new();

    let repo = &git_bug.repo;

    let author = make_author();
    let bug = create_bug(repo, author.clone(), "Test issue title".into(), "Body of the issue.".into())?;

    let bug_id = bug.id;

    let list_text = git_bug.run(&["bug"]);

    assert!(
        list_text.contains("Test issue title"),
        "git-bug bug did not list our issue.\nOutput:\n{list_text}"
    );

    let json = git_bug.show(&bug_id);

    assert_matches!(json["title"].as_str(), Some("Test issue title"));
    assert_matches!(json["status"].as_str(), Some("open"));
    assert_matches!(json["id"].as_str(), Some(id) if id == bug_id.to_string());

    let comments = json["comments"].as_array().context("git-bug show has no comments array")?;
    assert!(!comments.is_empty(), "expected at least one comment (the body)");
    assert_matches!(comments[0]["message"].as_str(), Some("Body of the issue."));

    Ok(())
}

#[test_with::executable(git-bug)]
fn git_bug_can_read_comment_and_closed_status() -> Result<()> {
    let git_bug = GitBug::new();

    let repo = &git_bug.repo;

    let author = make_author();
    let bug = create_bug(repo, author.clone(), "Issue with comment".into(), "Initial body.".into())?;

    let bug_id = bug.id;

    let bug = add_comment(repo, bug, author.clone(), "A follow-up comment.".into())?.bug;

    let _ = set_status(repo, bug, author, BugStatus::CLOSED)?;

    let json = git_bug.show(&bug_id);

    assert_matches!(json["status"].as_str(), Some("closed"));

    let comments = json["comments"].as_array().context("git-bug show has no comments array")?;
    assert_eq!(comments.len(), 2, "expected 2 comments (body + follow-up)");
    assert_matches!(comments[1]["message"].as_str(), Some("A follow-up comment."));

    Ok(())
}

#[test_with::executable(git-bug)]
fn comment_ids_match_git_bug_for_html_characters() -> Result<()> {
    let git_bug = GitBug::new();

    let repo = &git_bug.repo;

    git_bug.run(&["user", "new", "--non-interactive", "-n", "tester", "-e", "tester@gitarena.local"]);
    git_bug.run(&["bug", "new", "--non-interactive", "-t", "title", "-m", "body"]);

    let bug_id = list_bug_ids(repo)?.pop().context("git-bug did not create a bug")?;
    git_bug.run(&["bug", "comment", "new", &bug_id.to_string(), "--non-interactive", "-m", "a <b>tag</b> & stuff"]);

    let show = git_bug.show(&bug_id);
    let combined_id = show["comments"][1]["id"].as_str().context("git-bug comment has no id")?;

    let snapshot = load_bug(repo, &bug_id)?.snapshot();
    let comment_id = &snapshot.comments[0].id;

    // Combined IDs take the secondary (comment) ID at positions 1, 3, 5, 9 and every 5th from 14 (spec ids.md §5.1)
    let secondary: String = combined_id
        .chars()
        .enumerate()
        .filter(|(i, _)| matches!(i, 1 | 3 | 5 | 9) || (*i >= 10 && i % 5 == 4))
        .map(|(_, c)| c)
        .collect();

    assert!(
        comment_id.to_string().starts_with(&secondary),
        "comment id {comment_id} does not match git-bug combined id {combined_id}"
    );

    Ok(())
}

#[test_with::executable(git-bug, git)]
fn concurrent_edits_merged_by_git_bug_are_loaded() -> Result<()> {
    let upstream = GitBug::new();
    upstream.run(&["user", "new", "--non-interactive", "-n", "upstream", "-e", "upstream@gitarena.local"]);
    upstream.run(&["bug", "new", "--non-interactive", "-t", "title", "-m", "body"]);

    let bug_id = list_bug_ids(&upstream.repo)?.pop().context("git-bug did not create a bug")?;

    let local = GitBug::new();
    local.add_origin(&upstream);
    local.run(&["pull", "origin"]);
    local.run(&["user", "new", "--non-interactive", "-n", "local", "-e", "local@gitarena.local"]);

    local.run(&["bug", "comment", "new", &bug_id.to_string(), "--non-interactive", "-m", "from local"]);
    upstream.run(&["bug", "comment", "new", &bug_id.to_string(), "--non-interactive", "-m", "from upstream"]);
    local.run(&["pull", "origin"]);

    let bug = load_bug(&local.repo, &bug_id)?;
    assert_eq!(bug.packs.len(), 4, "expected create, two concurrent comments and the merge commit");

    let comments: Vec<String> = bug.snapshot().comments.into_iter().map(|comment| comment.body).collect();

    let show = local.show(&bug_id);
    let expected: Vec<String> = show["comments"].as_array().context("git-bug show has no comments array")?[1..]
        .iter()
        .map(|comment| comment["message"].as_str().map(str::to_owned))
        .collect::<Option<_>>()
        .context("git-bug comment has no message")?;

    assert_eq!(comments, expected);

    Ok(())
}

#[test_with::executable(git-bug, git)]
fn git_bug_can_read_our_merge_commit() -> Result<()> {
    let upstream = GitBug::new();
    upstream.run(&["user", "new", "--non-interactive", "-n", "upstream", "-e", "upstream@gitarena.local"]);
    upstream.run(&["bug", "new", "--non-interactive", "-t", "title", "-m", "body"]);

    let bug_id = list_bug_ids(&upstream.repo)?.pop().context("git-bug did not create a bug")?;

    let local = GitBug::new();
    local.add_origin(&upstream);
    local.run(&["pull", "origin"]);
    local.run(&["user", "new", "--non-interactive", "-n", "local", "-e", "local@gitarena.local"]);

    local.run(&["bug", "comment", "new", &bug_id.to_string(), "--non-interactive", "-m", "from local"]);
    upstream.run(&["bug", "comment", "new", &bug_id.to_string(), "--non-interactive", "-m", "from upstream"]);

    local.git(&["fetch", "origin", "refs/bugs/*:refs/remotes/origin/bugs/*"]);
    let remote_tip = local.repo.find_reference(format!("refs/remotes/origin/bugs/{bug_id}").as_str())?.id().detach();

    assert_eq!(merge_bug(&local.repo, &bug_id, remote_tip, &make_author())?, MergeOutcome::Merged);

    local.git(&["push", "origin", "refs/bugs/*:refs/bugs/*", "refs/identities/*:refs/identities/*"]);

    let messages: Vec<String> = upstream.show(&bug_id)["comments"]
        .as_array()
        .context("git-bug show has no comments array")?
        .iter()
        .map(|comment| comment["message"].as_str().map(str::to_owned))
        .collect::<Option<_>>()
        .context("git-bug comment has no message")?;

    assert_eq!(messages.len(), 3, "expected body and both comments, got {messages:?}");
    assert!(messages.contains(&"from local".to_owned()) && messages.contains(&"from upstream".to_owned()));

    Ok(())
}
