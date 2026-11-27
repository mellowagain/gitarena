use anyhow::{Context, Result};
use common::{Fixture, Transport, Visibility};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::collections::BTreeSet;

mod common;

fn comments(show: &Value) -> Result<Vec<(String, String)>> {
    show["comments"]
        .as_array()
        .context("git-bug show has no comments array")?
        .iter()
        .map(|comment| {
            let author = comment["author"]["name"].as_str().context("git-bug comment has no author")?;
            let message = comment["message"].as_str().context("git-bug comment has no message")?;

            Ok((author.to_owned(), message.to_owned()))
        })
        .collect()
}

fn string_set(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .map_or_default(|values| values.iter().filter_map(Value::as_str).map(ToOwned::to_owned).collect())
}

async fn issue_lifecycle(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let issue = fixture.owner.create_issue(&fixture.repo, "lifecycle", "issue body").await?;

    let git_bug = fixture.git_bug(Some(&fixture.owner), "bugs").await?;
    git_bug.pull().await?;
    assert_eq!(git_bug.show(&issue.git_bug_id).await?["status"], "open");

    fixture.owner.create_label(&fixture.repo, "bug").await?;
    fixture.owner.add_comment(&fixture.repo, issue.index, "first comment").await?;
    fixture.owner.update_issue(&fixture.repo, issue.index, json!({ "labelsAdd": ["bug"] })).await?;
    fixture
        .owner
        .update_issue(&fixture.repo, issue.index, json!({ "title": "lifecycle renamed" }))
        .await?;
    fixture.owner.add_comment(&fixture.repo, issue.index, "second comment").await?;
    fixture.owner.update_issue(&fixture.repo, issue.index, json!({ "status": "completed" })).await?;

    git_bug.pull().await?;

    let show = git_bug.show(&issue.git_bug_id).await?;
    let detail = fixture.owner.issue(&fixture.repo, issue.index).await?;
    let timeline = fixture.owner.timeline(&fixture.repo, issue.index).await?;

    let rest_comments = detail["comments"]
        .as_array()
        .context("issue detail has no comments")?
        .iter()
        .map(|comment| comment["body"].as_str().map(ToOwned::to_owned).context("comment has no body"))
        .collect::<Result<Vec<_>>>()?;

    let pulled = comments(&show)?;
    assert_eq!(pulled[0], ("alice".to_owned(), "issue body".to_owned()));
    assert_eq!(pulled[1..].iter().map(|(_, message)| message.clone()).collect::<Vec<_>>(), rest_comments);

    let kinds = timeline.iter().filter_map(|event| event["type"].as_str()).collect::<Vec<_>>();
    assert_eq!(kinds, vec!["created", "label_added", "title_changed", "status_changed"]);

    assert_eq!(show["title"], detail["issue"]["title"]);
    assert_eq!(show["title"], timeline[2]["newTitle"]);
    assert_eq!(show["status"], timeline[3]["status"]);
    assert_eq!(
        string_set(&show["labels"]),
        BTreeSet::from([timeline[1]["label"].as_str().context("label event has no label")?.to_owned()])
    );
    assert_eq!(string_set(&show["labels"]), string_set(&detail["issue"]["labels"]));
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn issue_lifecycle_http(pool: PgPool) -> Result<()> {
    issue_lifecycle(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn issue_lifecycle_ssh(pool: PgPool) -> Result<()> {
    issue_lifecycle(pool, Transport::Ssh).await
}

async fn multi_user(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let bob = fixture.user("bob").await?;
    let carol = fixture.user("carol").await?;

    let issue = fixture.owner.create_issue(&fixture.repo, "shared", "opened by alice").await?;
    bob.add_comment(&fixture.repo, issue.index, "comment by bob").await?;
    fixture.owner.add_comment(&fixture.repo, issue.index, "reply by alice").await?;
    fixture.owner.update_issue(&fixture.repo, issue.index, json!({ "status": "completed" })).await?;

    let git_bug = fixture.git_bug(Some(&carol), "carol").await?;
    git_bug.pull().await?;

    let show = git_bug.show(&issue.git_bug_id).await?;

    assert_eq!(
        comments(&show)?,
        vec![
            ("alice".to_owned(), "opened by alice".to_owned()),
            ("bob".to_owned(), "comment by bob".to_owned()),
            ("alice".to_owned(), "reply by alice".to_owned()),
        ]
    );
    assert_eq!(show["status"], "closed");

    let mut identities = BTreeSet::new();

    for reference in git_bug.local_refs("refs/identities").await? {
        let id = reference.trim_start_matches("refs/identities/");
        let identity = git_bug.identity(id).await?;

        identities.insert((
            identity["name"].as_str().context("identity has no name")?.to_owned(),
            identity["email"].as_str().context("identity has no email")?.to_owned(),
        ));
    }

    assert_eq!(
        identities,
        BTreeSet::from([
            ("alice".to_owned(), fixture.forge_mail(&pool, &fixture.owner).await?),
            ("bob".to_owned(), fixture.forge_mail(&pool, &bob).await?),
        ])
    );
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn multi_user_http(pool: PgPool) -> Result<()> {
    multi_user(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn multi_user_ssh(pool: PgPool) -> Result<()> {
    multi_user(pool, Transport::Ssh).await
}

async fn code_and_issues(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (local, _) = fixture.push_seed().await?;

    let first = fixture.owner.create_issue(&fixture.repo, "first", "body").await?;
    let second = fixture.owner.create_issue(&fixture.repo, "second", "body").await?;

    let clone = fixture.checkout(&fixture.owner, "clone");
    clone.clone_into(&fixture.owner_remote()).await?;

    let git_bug = fixture.git_bug(Some(&fixture.owner), "clone").await?;
    git_bug.pull().await?;

    let ids = |bugs: Vec<Value>| bugs.iter().filter_map(|bug| bug["id"].as_str().map(ToOwned::to_owned)).collect::<BTreeSet<_>>();
    let expected = BTreeSet::from([first.git_bug_id.clone(), second.git_bug_id.clone()]);

    assert_eq!(ids(git_bug.bugs().await?), expected);
    assert_eq!(clone.read_file("README.md").await?, "# test\n");

    let head = local.commit_file("later.txt", "later\n", "later commit").await?;
    local.ok(&["push", "--quiet"]).await?;

    clone.ok(&["pull", "--quiet", "--ff-only"]).await?;

    assert_eq!(clone.rev_parse("HEAD").await?, head);
    assert_eq!(ids(git_bug.bugs().await?), expected);
    assert_eq!(git_bug.show(&first.git_bug_id).await?["title"], "first");
    clone.fsck().await?;
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn code_and_issues_http(pool: PgPool) -> Result<()> {
    code_and_issues(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn code_and_issues_ssh(pool: PgPool) -> Result<()> {
    code_and_issues(pool, Transport::Ssh).await
}

async fn mixed_transport(pool: PgPool, first: Transport, second: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, Transport::Ssh, Visibility::Public).await?;
    let issue = fixture.owner.create_issue(&fixture.repo, "mixed", "issue body").await?;

    let git_bug = fixture.git_bug(Some(&fixture.owner), "bugs").await?;

    git_bug.set_remote(&fixture.server.remote(first, Some(&fixture.owner), &fixture.repo)).await?;
    git_bug.pull().await?;

    fixture.owner.add_comment(&fixture.repo, issue.index, "between pulls").await?;

    git_bug.set_remote(&fixture.server.remote(second, Some(&fixture.owner), &fixture.repo)).await?;
    git_bug.pull().await?;

    let messages = comments(&git_bug.show(&issue.git_bug_id).await?)?
        .into_iter()
        .map(|(_, message)| message)
        .collect::<Vec<_>>();
    assert_eq!(messages, vec!["issue body", "between pulls"]);
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_over_http_then_ssh(pool: PgPool) -> Result<()> {
    mixed_transport(pool, Transport::Http, Transport::Ssh).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn pull_over_ssh_then_http(pool: PgPool) -> Result<()> {
    mixed_transport(pool, Transport::Ssh, Transport::Http).await
}

async fn many_issues(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let mut issues = Vec::new();

    for index in 0..50 {
        let issue = fixture
            .owner
            .create_issue(&fixture.repo, &format!("issue {index}"), &format!("body {index}"))
            .await?;
        fixture.owner.add_comment(&fixture.repo, issue.index, &format!("comment {index}")).await?;

        issues.push(issue);
    }

    let git_bug = fixture.git_bug(Some(&fixture.owner), "bugs").await?;
    git_bug.pull().await?;

    assert_eq!(git_bug.bugs().await?.len(), 50);

    for (index, issue) in issues.iter().enumerate().step_by(7) {
        let show = git_bug.show(&issue.git_bug_id).await?;

        assert_eq!(show["title"], format!("issue {index}").as_str());
        assert_eq!(
            comments(&show)?.into_iter().map(|(_, message)| message).collect::<Vec<_>>(),
            vec![format!("body {index}"), format!("comment {index}")]
        );
    }

    git_bug.git.fsck().await?;
    Ok(())
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn many_issues_http(pool: PgPool) -> Result<()> {
    many_issues(pool, Transport::Http).await
}

#[test_with::executable(git-bug)]
#[sqlx::test(migrations = "../migrations")]
async fn many_issues_ssh(pool: PgPool) -> Result<()> {
    many_issues(pool, Transport::Ssh).await
}
