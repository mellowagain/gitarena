use anyhow::Result;
use common::scenarios;
use common::{Fixture, Git, Protocol, Transport, Visibility};
use pretty_assertions::assert_eq;
use sqlx::PgPool;

mod common;

#[sqlx::test(migrations = "../migrations")]
async fn clone_empty_repo_v1(pool: PgPool) -> Result<()> {
    scenarios::clone_empty_repo(pool, Transport::Http, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn clone_empty_repo_v2(pool: PgPool) -> Result<()> {
    scenarios::clone_empty_repo(pool, Transport::Http, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn clone_non_empty_repo_v1(pool: PgPool) -> Result<()> {
    scenarios::clone_non_empty_repo(pool, Transport::Http, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn clone_non_empty_repo_v2(pool: PgPool) -> Result<()> {
    scenarios::clone_non_empty_repo(pool, Transport::Http, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn fetch_after_server_change_v1(pool: PgPool) -> Result<()> {
    scenarios::fetch_after_server_change(pool, Transport::Http, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn fetch_after_server_change_v2(pool: PgPool) -> Result<()> {
    scenarios::fetch_after_server_change(pool, Transport::Http, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn ls_remote_lists_all_refs_v1(pool: PgPool) -> Result<()> {
    scenarios::ls_remote_lists_all_refs(pool, Transport::Http, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn ls_remote_lists_all_refs_v2(pool: PgPool) -> Result<()> {
    scenarios::ls_remote_lists_all_refs(pool, Transport::Http, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn ls_remote_peels_annotated_tags_v1(pool: PgPool) -> Result<()> {
    scenarios::ls_remote_peels_annotated_tags(pool, Transport::Http, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn ls_remote_peels_annotated_tags_v2(pool: PgPool) -> Result<()> {
    scenarios::ls_remote_peels_annotated_tags(pool, Transport::Http, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn owner_can_clone_and_push_private_repo_v1(pool: PgPool) -> Result<()> {
    scenarios::owner_can_clone_and_push_private_repo(pool, Transport::Http, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn owner_can_clone_and_push_private_repo_v2(pool: PgPool) -> Result<()> {
    scenarios::owner_can_clone_and_push_private_repo(pool, Transport::Http, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn other_user_can_clone_public_repo_v1(pool: PgPool) -> Result<()> {
    scenarios::other_user_can_clone_public_repo(pool, Transport::Http, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn other_user_can_clone_public_repo_v2(pool: PgPool) -> Result<()> {
    scenarios::other_user_can_clone_public_repo(pool, Transport::Http, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn other_user_cannot_clone_private_repo_v1(pool: PgPool) -> Result<()> {
    scenarios::other_user_cannot_clone_private_repo(pool, Transport::Http, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn other_user_cannot_clone_private_repo_v2(pool: PgPool) -> Result<()> {
    scenarios::other_user_cannot_clone_private_repo(pool, Transport::Http, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn nonexistent_repo_fails_v1(pool: PgPool) -> Result<()> {
    scenarios::nonexistent_repo_fails(pool, Transport::Http, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn nonexistent_repo_fails_v2(pool: PgPool) -> Result<()> {
    scenarios::nonexistent_repo_fails(pool, Transport::Http, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn initial_push(pool: PgPool) -> Result<()> {
    scenarios::initial_push(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn push_new_branch(pool: PgPool) -> Result<()> {
    scenarios::push_new_branch(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn push_tags(pool: PgPool) -> Result<()> {
    scenarios::push_tags(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn force_push(pool: PgPool) -> Result<()> {
    scenarios::force_push(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn delete_branch(pool: PgPool) -> Result<()> {
    scenarios::delete_branch(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn delete_tag(pool: PgPool) -> Result<()> {
    scenarios::delete_tag(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn delete_multiple_refs_in_one_push(pool: PgPool) -> Result<()> {
    scenarios::delete_multiple_refs_in_one_push(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn other_user_cannot_push_to_public_repo(pool: PgPool) -> Result<()> {
    scenarios::other_user_cannot_push_to_public_repo(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn other_user_cannot_push_to_private_repo(pool: PgPool) -> Result<()> {
    scenarios::other_user_cannot_push_to_private_repo(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn nonexistent_repo_push_fails(pool: PgPool) -> Result<()> {
    scenarios::nonexistent_repo_push_fails(pool, Transport::Http).await
}

#[sqlx::test(migrations = "../migrations")]
async fn push_with_secret_is_rejected(pool: PgPool) -> Result<()> {
    scenarios::push_with_secret_is_rejected(pool, Transport::Http).await
}

async fn anonymous_clone_public_repo(pool: PgPool, protocol: Protocol) -> Result<()> {
    let fixture = Fixture::new(&pool, Transport::Http, Visibility::Public).await?;
    let (_, head) = fixture.push_seed().await?;

    let clone = Git::new(&fixture.path("clone")).with_protocol(protocol);
    clone.clone_into(&fixture.remote(None)).await?;

    assert_eq!(clone.rev_parse("HEAD").await?, head);
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn anonymous_clone_public_repo_v1(pool: PgPool) -> Result<()> {
    anonymous_clone_public_repo(pool, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn anonymous_clone_public_repo_v2(pool: PgPool) -> Result<()> {
    anonymous_clone_public_repo(pool, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn anonymous_clone_private_repo_fails(pool: PgPool) -> Result<()> {
    let fixture = Fixture::new(&pool, Transport::Http, Visibility::Private).await?;
    fixture.push_seed().await?;

    let clone = Git::new(&fixture.path("clone"));
    clone.init().await?;

    let stderr = clone.fails(&["fetch", &fixture.remote(None), "main"]).await?;
    assert!(stderr.contains("could not read Username"), "{stderr}");
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn anonymous_push_fails(pool: PgPool) -> Result<()> {
    let fixture = Fixture::new(&pool, Transport::Http, Visibility::Public).await?;
    let (_, head) = fixture.push_seed().await?;

    let clone = Git::new(&fixture.path("clone"));
    clone.clone_into(&fixture.remote(None)).await?;
    clone.commit_file("anonymous.txt", "anonymous\n", "anonymous commit").await?;

    let stderr = clone.fails(&["push", "origin", "main"]).await?;
    assert!(stderr.contains("could not read Username"), "{stderr}");
    assert_eq!(fixture.refs().await?.get("refs/heads/main"), Some(&head));
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn wrong_password_fails(pool: PgPool) -> Result<()> {
    let fixture = Fixture::new(&pool, Transport::Http, Visibility::Private).await?;
    fixture.push_seed().await?;

    let remote = format!("http://alice:wrong-password@127.0.0.1:{}/{}.git", fixture.server.http_port, fixture.repo);

    let clone = Git::new(&fixture.path("clone"));
    clone.init().await?;

    let stderr = clone.fails(&["fetch", &remote, "main"]).await?;
    assert!(stderr.contains("Authentication failed") || stderr.contains("401"), "{stderr}");
    Ok(())
}
