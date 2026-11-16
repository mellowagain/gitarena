use anyhow::{Context, Result};
use common::{Fixture, GitBug, Issue, Transport, Visibility};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use sqlx::PgPool;

mod common;

async fn pulled_issue(pool: &PgPool, transport: Transport) -> Result<(Fixture, Issue, GitBug)> {
    let fixture = Fixture::new(pool, transport, Visibility::Public).await?;
    let issue = fixture.owner.create_issue(&fixture.repo, "first issue", "issue body").await?;

    let git_bug = fixture.git_bug(Some(&fixture.owner), "bugs").await?;
    git_bug.pull().await?;

    Ok((fixture, issue, git_bug))
}

fn messages(show: &Value) -> Result<Vec<String>> {
    show["comments"]
        .as_array()
        .context("git-bug show has no comments array")?
        .iter()
        .map(|comment| comment["message"].as_str().map(ToOwned::to_owned).context("git-bug comment has no message"))
        .collect()
}

fn labels(show: &Value) -> Vec<&str> {
    show["labels"]
        .as_array()
        .map_or_default(|labels| labels.iter().filter_map(Value::as_str).collect())
}

async fn pull_issue(pool: PgPool, transport: Transport) -> Result<()> {
    let (fixture, issue, git_bug) = pulled_issue(&pool, transport).await?;

    let bugs = git_bug.bugs().await?;
    assert_eq!(bugs.len(), 1);

    let show = git_bug.show(&issue.git_bug_id).await?;
    assert_eq!(show["id"], issue.git_bug_id.as_str());
    assert_eq!(show["title"], "first issue");
    assert_eq!(show["status"], "open");
    assert_eq!(messages(&show)?, vec!["issue body"]);
    assert_eq!(show["author"]["name"], "alice");

    let author_id = show["author"]["id"].as_str().context("git-bug author has no id")?;
    let identity = git_bug.identity(author_id).await?;

    assert_eq!(identity["name"], "alice");
    assert_eq!(identity["email"], fixture.forge_mail(&pool, &fixture.owner).await?.as_str());
    assert_eq!(git_bug.local_refs("refs/identities").await?, vec![format!("refs/identities/{author_id}")]);
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_issue_http(pool: PgPool) -> Result<()> {
    pull_issue(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_issue_ssh(pool: PgPool) -> Result<()> {
    pull_issue(pool, Transport::Ssh).await
}

async fn pull_added_comment(pool: PgPool, transport: Transport) -> Result<()> {
    let (fixture, issue, git_bug) = pulled_issue(&pool, transport).await?;

    fixture.owner.add_comment(&fixture.repo, issue.index, "a comment").await?;
    git_bug.pull().await?;

    assert_eq!(messages(&git_bug.show(&issue.git_bug_id).await?)?, vec!["issue body", "a comment"]);
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_added_comment_http(pool: PgPool) -> Result<()> {
    pull_added_comment(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_added_comment_ssh(pool: PgPool) -> Result<()> {
    pull_added_comment(pool, Transport::Ssh).await
}

async fn pull_edited_comment(pool: PgPool, transport: Transport) -> Result<()> {
    let (fixture, issue, git_bug) = pulled_issue(&pool, transport).await?;

    let comment = fixture.owner.add_comment(&fixture.repo, issue.index, "a comment").await?;
    git_bug.pull().await?;

    fixture.owner.edit_comment(&fixture.repo, issue.index, &comment, "an edited comment").await?;
    git_bug.pull().await?;

    assert_eq!(messages(&git_bug.show(&issue.git_bug_id).await?)?, vec!["issue body", "an edited comment"]);
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_edited_comment_http(pool: PgPool) -> Result<()> {
    pull_edited_comment(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_edited_comment_ssh(pool: PgPool) -> Result<()> {
    pull_edited_comment(pool, Transport::Ssh).await
}

async fn pull_status_changes(pool: PgPool, transport: Transport) -> Result<()> {
    let (fixture, issue, git_bug) = pulled_issue(&pool, transport).await?;

    fixture.owner.update_issue(&fixture.repo, issue.index, json!({ "status": "completed" })).await?;
    git_bug.pull().await?;
    assert_eq!(git_bug.show(&issue.git_bug_id).await?["status"], "closed");

    fixture.owner.update_issue(&fixture.repo, issue.index, json!({ "status": "open" })).await?;
    git_bug.pull().await?;
    assert_eq!(git_bug.show(&issue.git_bug_id).await?["status"], "open");
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_status_changes_http(pool: PgPool) -> Result<()> {
    pull_status_changes(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_status_changes_ssh(pool: PgPool) -> Result<()> {
    pull_status_changes(pool, Transport::Ssh).await
}

async fn pull_title_change(pool: PgPool, transport: Transport) -> Result<()> {
    let (fixture, issue, git_bug) = pulled_issue(&pool, transport).await?;

    fixture
        .owner
        .update_issue(&fixture.repo, issue.index, json!({ "title": "renamed issue" }))
        .await?;
    git_bug.pull().await?;

    assert_eq!(git_bug.show(&issue.git_bug_id).await?["title"], "renamed issue");
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_title_change_http(pool: PgPool) -> Result<()> {
    pull_title_change(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_title_change_ssh(pool: PgPool) -> Result<()> {
    pull_title_change(pool, Transport::Ssh).await
}

async fn pull_label_changes(pool: PgPool, transport: Transport) -> Result<()> {
    let (fixture, issue, git_bug) = pulled_issue(&pool, transport).await?;

    fixture.owner.create_label(&fixture.repo, "bug").await?;
    fixture.owner.create_label(&fixture.repo, "docs").await?;

    fixture
        .owner
        .update_issue(&fixture.repo, issue.index, json!({ "labelsAdd": ["bug", "docs"] }))
        .await?;
    git_bug.pull().await?;
    assert_eq!(labels(&git_bug.show(&issue.git_bug_id).await?), vec!["bug", "docs"]);

    fixture
        .owner
        .update_issue(&fixture.repo, issue.index, json!({ "labelsRemove": ["bug"] }))
        .await?;
    git_bug.pull().await?;
    assert_eq!(labels(&git_bug.show(&issue.git_bug_id).await?), vec!["docs"]);
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_label_changes_http(pool: PgPool) -> Result<()> {
    pull_label_changes(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_label_changes_ssh(pool: PgPool) -> Result<()> {
    pull_label_changes(pool, Transport::Ssh).await
}

async fn deleted_issue_ref_disappears(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let issue = fixture.owner.create_issue(&fixture.repo, "doomed", "body").await?;
    let bug_ref = format!("refs/bugs/{}", issue.git_bug_id);

    assert!(fixture.refs().await?.contains_key(&bug_ref));

    fixture.owner.delete_issue(&fixture.repo, issue.index).await?;

    assert!(!fixture.refs().await?.contains_key(&bug_ref));
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn deleted_issue_ref_disappears_http(pool: PgPool) -> Result<()> {
    deleted_issue_ref_disappears(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn deleted_issue_ref_disappears_ssh(pool: PgPool) -> Result<()> {
    deleted_issue_ref_disappears(pool, Transport::Ssh).await
}

async fn push_of_local_bug_is_rejected(pool: PgPool, transport: Transport) -> Result<()> {
    let (fixture, _, git_bug) = pulled_issue(&pool, transport).await?;
    let before = fixture.refs().await?;

    git_bug.create_identity("local").await?;
    git_bug.create_bug("local bug").await?;
    git_bug.fails(&["push", "origin"]).await?;

    assert_eq!(fixture.refs().await?, before);
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn push_of_local_bug_is_rejected_http(pool: PgPool) -> Result<()> {
    push_of_local_bug_is_rejected(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn push_of_local_bug_is_rejected_ssh(pool: PgPool) -> Result<()> {
    push_of_local_bug_is_rejected(pool, Transport::Ssh).await
}

async fn push_of_local_identity_is_rejected(pool: PgPool, transport: Transport) -> Result<()> {
    let (fixture, _, git_bug) = pulled_issue(&pool, transport).await?;
    let before = fixture.refs().await?;

    git_bug.create_identity("local").await?;
    git_bug.fails(&["push", "origin"]).await?;

    assert_eq!(fixture.refs().await?, before);
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn push_of_local_identity_is_rejected_http(pool: PgPool) -> Result<()> {
    push_of_local_identity_is_rejected(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn push_of_local_identity_is_rejected_ssh(pool: PgPool) -> Result<()> {
    push_of_local_identity_is_rejected(pool, Transport::Ssh).await
}

async fn plain_git_push_of_bug_refs_is_rejected(pool: PgPool, transport: Transport) -> Result<()> {
    let (fixture, _, git_bug) = pulled_issue(&pool, transport).await?;
    let before = fixture.refs().await?;

    git_bug.create_identity("local").await?;
    git_bug.create_bug("local bug").await?;

    git_bug.git.fails(&["push", "origin", "refs/bugs/*:refs/bugs/*"]).await?;
    git_bug.git.fails(&["push", "origin", "refs/identities/*:refs/identities/*"]).await?;

    assert_eq!(fixture.refs().await?, before);
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn plain_git_push_of_bug_refs_is_rejected_http(pool: PgPool) -> Result<()> {
    plain_git_push_of_bug_refs_is_rejected(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn plain_git_push_of_bug_refs_is_rejected_ssh(pool: PgPool) -> Result<()> {
    plain_git_push_of_bug_refs_is_rejected(pool, Transport::Ssh).await
}

async fn other_user_cannot_pull_private_repo(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Private).await?;
    fixture.owner.create_issue(&fixture.repo, "secret issue", "body").await?;
    let bob = fixture.user("bob").await?;

    let git_bug = fixture.git_bug(Some(&bob), "bugs").await?;
    git_bug.fails(&["pull", "origin"]).await?;

    assert_eq!(git_bug.local_refs("refs/bugs").await?, Vec::<String>::new());
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn other_user_cannot_pull_private_repo_http(pool: PgPool) -> Result<()> {
    other_user_cannot_pull_private_repo(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn other_user_cannot_pull_private_repo_ssh(pool: PgPool) -> Result<()> {
    other_user_cannot_pull_private_repo(pool, Transport::Ssh).await
}

async fn plain_clone_does_not_fetch_bug_refs(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    fixture.push_seed().await?;
    let issue = fixture.owner.create_issue(&fixture.repo, "issue", "body").await?;

    assert!(fixture.refs().await?.contains_key(&format!("refs/bugs/{}", issue.git_bug_id)));

    let clone = fixture.checkout(&fixture.owner, "clone");
    clone.clone_into(&fixture.owner_remote()).await?;

    assert_eq!(clone.ok(&["for-each-ref", "refs/bugs", "refs/identities"]).await?, "");
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn plain_clone_does_not_fetch_bug_refs_http(pool: PgPool) -> Result<()> {
    plain_clone_does_not_fetch_bug_refs(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn plain_clone_does_not_fetch_bug_refs_ssh(pool: PgPool) -> Result<()> {
    plain_clone_does_not_fetch_bug_refs(pool, Transport::Ssh).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn anonymous_pull_public_repo(pool: PgPool) -> Result<()> {
    let fixture = Fixture::new(&pool, Transport::Http, Visibility::Public).await?;
    let issue = fixture.owner.create_issue(&fixture.repo, "public issue", "body").await?;

    let git_bug = fixture.git_bug(None, "bugs").await?;
    git_bug.pull().await?;

    assert_eq!(git_bug.show(&issue.git_bug_id).await?["title"], "public issue");
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn anonymous_pull_private_repo_fails(pool: PgPool) -> Result<()> {
    let fixture = Fixture::new(&pool, Transport::Http, Visibility::Private).await?;
    fixture.owner.create_issue(&fixture.repo, "secret issue", "body").await?;

    let git_bug = fixture.git_bug(None, "bugs").await?;
    git_bug.fails(&["pull", "origin"]).await?;

    assert_eq!(git_bug.local_refs("refs/bugs").await?, Vec::<String>::new());
    Ok(())
}
