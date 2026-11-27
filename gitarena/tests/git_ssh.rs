use anyhow::Result;
use common::scenarios;
use common::{Fixture, Git, Protocol, Transport, Visibility, generate_ssh_key};
use sqlx::PgPool;

mod common;

#[sqlx::test(migrations = "../migrations")]
async fn clone_empty_repo_v1(pool: PgPool) -> Result<()> {
    scenarios::clone_empty_repo(pool, Transport::Ssh, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn clone_empty_repo_v2(pool: PgPool) -> Result<()> {
    scenarios::clone_empty_repo(pool, Transport::Ssh, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn clone_non_empty_repo_v1(pool: PgPool) -> Result<()> {
    scenarios::clone_non_empty_repo(pool, Transport::Ssh, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn clone_non_empty_repo_v2(pool: PgPool) -> Result<()> {
    scenarios::clone_non_empty_repo(pool, Transport::Ssh, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn fetch_after_server_change_v1(pool: PgPool) -> Result<()> {
    scenarios::fetch_after_server_change(pool, Transport::Ssh, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn fetch_after_server_change_v2(pool: PgPool) -> Result<()> {
    scenarios::fetch_after_server_change(pool, Transport::Ssh, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn ls_remote_lists_all_refs_v1(pool: PgPool) -> Result<()> {
    scenarios::ls_remote_lists_all_refs(pool, Transport::Ssh, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn ls_remote_lists_all_refs_v2(pool: PgPool) -> Result<()> {
    scenarios::ls_remote_lists_all_refs(pool, Transport::Ssh, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn ls_remote_peels_annotated_tags_v1(pool: PgPool) -> Result<()> {
    scenarios::ls_remote_peels_annotated_tags(pool, Transport::Ssh, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn ls_remote_peels_annotated_tags_v2(pool: PgPool) -> Result<()> {
    scenarios::ls_remote_peels_annotated_tags(pool, Transport::Ssh, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn owner_can_clone_and_push_private_repo_v1(pool: PgPool) -> Result<()> {
    scenarios::owner_can_clone_and_push_private_repo(pool, Transport::Ssh, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn owner_can_clone_and_push_private_repo_v2(pool: PgPool) -> Result<()> {
    scenarios::owner_can_clone_and_push_private_repo(pool, Transport::Ssh, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn other_user_can_clone_public_repo_v1(pool: PgPool) -> Result<()> {
    scenarios::other_user_can_clone_public_repo(pool, Transport::Ssh, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn other_user_can_clone_public_repo_v2(pool: PgPool) -> Result<()> {
    scenarios::other_user_can_clone_public_repo(pool, Transport::Ssh, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn other_user_cannot_clone_private_repo_v1(pool: PgPool) -> Result<()> {
    scenarios::other_user_cannot_clone_private_repo(pool, Transport::Ssh, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn other_user_cannot_clone_private_repo_v2(pool: PgPool) -> Result<()> {
    scenarios::other_user_cannot_clone_private_repo(pool, Transport::Ssh, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn nonexistent_repo_fails_v1(pool: PgPool) -> Result<()> {
    scenarios::nonexistent_repo_fails(pool, Transport::Ssh, Protocol::V1).await
}

#[sqlx::test(migrations = "../migrations")]
async fn nonexistent_repo_fails_v2(pool: PgPool) -> Result<()> {
    scenarios::nonexistent_repo_fails(pool, Transport::Ssh, Protocol::V2).await
}

#[sqlx::test(migrations = "../migrations")]
async fn initial_push(pool: PgPool) -> Result<()> {
    scenarios::initial_push(pool, Transport::Ssh).await
}

#[sqlx::test(migrations = "../migrations")]
async fn push_new_branch(pool: PgPool) -> Result<()> {
    scenarios::push_new_branch(pool, Transport::Ssh).await
}

#[sqlx::test(migrations = "../migrations")]
async fn push_tags(pool: PgPool) -> Result<()> {
    scenarios::push_tags(pool, Transport::Ssh).await
}

#[sqlx::test(migrations = "../migrations")]
async fn force_push(pool: PgPool) -> Result<()> {
    scenarios::force_push(pool, Transport::Ssh).await
}

#[sqlx::test(migrations = "../migrations")]
async fn delete_branch(pool: PgPool) -> Result<()> {
    scenarios::delete_branch(pool, Transport::Ssh).await
}

#[sqlx::test(migrations = "../migrations")]
async fn delete_tag(pool: PgPool) -> Result<()> {
    scenarios::delete_tag(pool, Transport::Ssh).await
}

#[sqlx::test(migrations = "../migrations")]
async fn delete_multiple_refs_in_one_push(pool: PgPool) -> Result<()> {
    scenarios::delete_multiple_refs_in_one_push(pool, Transport::Ssh).await
}

#[sqlx::test(migrations = "../migrations")]
async fn other_user_cannot_push_to_public_repo(pool: PgPool) -> Result<()> {
    scenarios::other_user_cannot_push_to_public_repo(pool, Transport::Ssh).await
}

#[sqlx::test(migrations = "../migrations")]
async fn other_user_cannot_push_to_private_repo(pool: PgPool) -> Result<()> {
    scenarios::other_user_cannot_push_to_private_repo(pool, Transport::Ssh).await
}

#[sqlx::test(migrations = "../migrations")]
async fn nonexistent_repo_push_fails(pool: PgPool) -> Result<()> {
    scenarios::nonexistent_repo_push_fails(pool, Transport::Ssh).await
}

#[sqlx::test(migrations = "../migrations")]
async fn push_with_secret_is_rejected(pool: PgPool) -> Result<()> {
    scenarios::push_with_secret_is_rejected(pool, Transport::Ssh).await
}

#[sqlx::test(migrations = "../migrations")]
async fn unregistered_key_fails(pool: PgPool) -> Result<()> {
    let fixture = Fixture::new(&pool, Transport::Ssh, Visibility::Public).await?;
    fixture.push_seed().await?;

    let (key, _) = generate_ssh_key(fixture.dir.path(), "unregistered").await?;

    let clone = Git::new(&fixture.path("clone")).with_ssh_key(&key);
    clone.init().await?;

    let stderr = clone.fails(&["fetch", &fixture.remote(None), "main"]).await?;
    assert!(stderr.contains("Permission denied"), "{stderr}");
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn no_key_fails(pool: PgPool) -> Result<()> {
    let fixture = Fixture::new(&pool, Transport::Ssh, Visibility::Public).await?;
    fixture.push_seed().await?;

    let clone = Git::new(&fixture.path("clone"));
    clone.init().await?;

    let stderr = clone.fails(&["fetch", &fixture.remote(None), "main"]).await?;
    assert!(stderr.contains("Permission denied"), "{stderr}");
    Ok(())
}
