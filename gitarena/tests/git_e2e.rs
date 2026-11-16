use anyhow::Result;
use common::{Fixture, Transport, Visibility};
use pretty_assertions::assert_eq;
use sqlx::PgPool;

mod common;

async fn full_lifecycle(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (first, _) = fixture.push_seed().await?;

    let second = fixture.checkout(&fixture.owner, "second");
    second.clone_into(&fixture.owner_remote()).await?;

    let from_first = first.commit_file("first.txt", "first\n", "commit from first").await?;
    first.ok(&["push", "--quiet"]).await?;

    second.commit_file("second.txt", "second\n", "commit from second").await?;

    let stderr = second.fails(&["push", "origin", "main"]).await?;
    assert!(stderr.contains("[rejected]"), "{stderr}");

    second.ok(&["pull", "--quiet", "--rebase"]).await?;
    assert_eq!(second.rev_parse("HEAD~1").await?, from_first);

    let rebased = second.rev_parse("HEAD").await?;
    second.ok(&["push", "--quiet"]).await?;

    first.ok(&["pull", "--quiet", "--ff-only"]).await?;

    assert_eq!(first.rev_parse("HEAD").await?, rebased);
    assert_eq!(first.read_file("second.txt").await?, "second\n");
    assert_eq!(fixture.refs().await?.get("refs/heads/main"), Some(&rebased));

    first.fsck().await?;
    second.fsck().await?;
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn full_lifecycle_http(pool: PgPool) -> Result<()> {
    full_lifecycle(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn full_lifecycle_ssh(pool: PgPool) -> Result<()> {
    full_lifecycle(pool, Transport::Ssh).await
}

async fn branching_workflow(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (local, _) = fixture.push_seed().await?;

    local.ok(&["switch", "--quiet", "--create", "feature"]).await?;
    local.commit_file("feature.txt", "feature\n", "add feature").await?;
    local.ok(&["push", "--quiet", "--set-upstream", "origin", "feature"]).await?;

    local.ok(&["switch", "--quiet", "main"]).await?;
    local.commit_file("main.txt", "main\n", "work on main").await?;
    local.ok(&["merge", "--quiet", "--no-ff", "--message", "merge feature", "feature"]).await?;
    local.ok(&["push", "--quiet", "origin", "main"]).await?;

    local.ok(&["push", "--quiet", "origin", "--delete", "feature"]).await?;
    local.ok(&["branch", "--quiet", "--delete", "feature"]).await?;

    local.ok(&["tag", "--annotate", "--message", "release 1.0", "v1.0"]).await?;
    local.ok(&["push", "--quiet", "origin", "v1.0"]).await?;

    let head = local.rev_parse("HEAD").await?;
    let refs = fixture.refs().await?;

    assert_eq!(refs.get("refs/heads/feature"), None);
    assert_eq!(refs.get("refs/heads/main"), Some(&head));
    assert_eq!(refs.get("refs/tags/v1.0"), Some(&local.rev_parse("v1.0").await?));

    let clone = fixture.checkout(&fixture.owner, "clone");
    clone.clone_into(&fixture.owner_remote()).await?;

    assert_eq!(clone.rev_parse("HEAD").await?, head);
    assert_eq!(clone.rev_parse("v1.0^{commit}").await?, head);
    assert_eq!(clone.rev_count("HEAD").await?, local.rev_count("HEAD").await?);
    assert_eq!(
        clone.ok(&["rev-list", "--parents", "--max-count=1", "HEAD"]).await?.split_whitespace().count(),
        3
    );
    assert_eq!(clone.ok(&["branch", "--remotes", "--list", "origin/feature"]).await?, "");
    assert_eq!(clone.read_file("feature.txt").await?, "feature\n");
    clone.fsck().await?;
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn branching_workflow_http(pool: PgPool) -> Result<()> {
    branching_workflow(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn branching_workflow_ssh(pool: PgPool) -> Result<()> {
    branching_workflow(pool, Transport::Ssh).await
}

async fn collaboration(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (alice, _) = fixture.push_seed().await?;

    alice.ok(&["push", "--quiet", "origin", "main:refs/heads/other"]).await?;

    let bob = fixture.user("bob").await?;
    let clone = fixture.checkout(&bob, "bob");
    clone
        .clone_into_with(&fixture.remote(Some(&bob)), &["--single-branch", "--branch", "main"])
        .await?;

    let rejected = clone.commit_file("bob.txt", "bob\n", "bob's change").await?;
    clone.fails(&["push", "origin", "main"]).await?;
    assert!(fixture.refs().await?.values().all(|oid| oid != &rejected));

    clone.ok(&["reset", "--quiet", "--hard", "origin/main"]).await?;

    for round in 0..3 {
        let count = clone.rev_count("origin/main").await?;

        alice
            .commit_file(&format!("round-{round}.txt"), &"line\n".repeat(1000 * (round + 1)), &format!("round {round}"))
            .await?;
        let head = alice.commit_file("shared.txt", &format!("{round}\n").repeat(500), "update shared").await?;
        alice.ok(&["push", "--quiet"]).await?;

        clone.ok(&["fetch", "--quiet"]).await?;

        assert_eq!(clone.rev_parse("origin/main").await?, head);
        assert_eq!(clone.rev_count("origin/main").await?, count + 2);
    }

    assert_eq!(
        clone.ok(&["for-each-ref", "--format=%(refname)", "refs/remotes"]).await?.trim(),
        "refs/remotes/origin/main"
    );

    clone.ok(&["merge", "--quiet", "--ff-only", "origin/main"]).await?;
    assert_eq!(clone.read_file("shared.txt").await?, "2\n".repeat(500));
    clone.fsck().await?;
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn collaboration_http(pool: PgPool) -> Result<()> {
    collaboration(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn collaboration_ssh(pool: PgPool) -> Result<()> {
    collaboration(pool, Transport::Ssh).await
}

async fn large_repo_round_trip(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;

    let local = fixture.checkout(&fixture.owner, "large");
    local.fast_import(500, 3, 4 * 1024 * 1024).await?;
    local.ok(&["push", "--quiet", &fixture.owner_remote(), "main"]).await?;

    let clone = fixture.checkout(&fixture.owner, "clone");
    clone.clone_into(&fixture.owner_remote()).await?;

    assert_eq!(clone.rev_count("HEAD").await?, 500);
    assert_eq!(clone.rev_parse("HEAD").await?, local.rev_parse("main").await?);
    assert_eq!(clone.rev_parse("HEAD^{tree}").await?, local.rev_parse("main^{tree}").await?);
    clone.fsck().await?;
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn large_repo_round_trip_http(pool: PgPool) -> Result<()> {
    large_repo_round_trip(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn large_repo_round_trip_ssh(pool: PgPool) -> Result<()> {
    large_repo_round_trip(pool, Transport::Ssh).await
}

async fn mixed_transport(pool: PgPool, push_over: Transport, clone_over: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, push_over, Visibility::Public).await?;
    let (local, _) = fixture.push_seed().await?;
    let head = local.commit_file("more.txt", "more\n", "second commit").await?;
    local.ok(&["push", "--quiet"]).await?;

    let clone = fixture.checkout(&fixture.owner, "clone");
    clone
        .clone_into(&fixture.server.remote(clone_over, Some(&fixture.owner), &fixture.repo))
        .await?;

    assert_eq!(clone.rev_parse("HEAD").await?, head);

    let from_clone = clone.commit_file("from-clone.txt", "clone\n", "commit from clone").await?;
    clone.ok(&["push", "--quiet"]).await?;

    local.ok(&["pull", "--quiet", "--ff-only"]).await?;
    assert_eq!(local.rev_parse("HEAD").await?, from_clone);

    clone.fsck().await?;
    local.fsck().await?;
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn push_over_http_clone_over_ssh(pool: PgPool) -> Result<()> {
    mixed_transport(pool, Transport::Http, Transport::Ssh).await
}

#[sqlx::test(migrations = "../migrations")]
async fn push_over_ssh_clone_over_http(pool: PgPool) -> Result<()> {
    mixed_transport(pool, Transport::Ssh, Transport::Http).await
}
