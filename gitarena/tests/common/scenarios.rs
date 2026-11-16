use super::{Fixture, Protocol, Transport, Visibility};
use anyhow::Result;
use pretty_assertions::assert_eq;
use sqlx::PgPool;

pub const SECRET: &str =
    "sk_gap_1955bd1d7e0759d3c7d41117fd42661e6ab51f8dd6d437cf6fc23bde26f32927179a9a7dfcc66706041ad94020fb132ef1cef931cc3f7825fa63661092fa57a4";

pub async fn clone_empty_repo(pool: PgPool, transport: Transport, protocol: Protocol) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;

    let clone = fixture.checkout(&fixture.owner, "clone").with_protocol(protocol);
    clone.clone_into(&fixture.owner_remote()).await?;

    assert_eq!(clone.ok(&["symbolic-ref", "HEAD"]).await?.trim(), "refs/heads/main");
    assert_eq!(clone.ok(&["rev-list", "--all"]).await?, "");
    Ok(())
}

pub async fn initial_push(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;

    let (_, head) = fixture.push_seed().await?;
    let refs = fixture.refs().await?;

    assert_eq!(refs.get("refs/heads/main"), Some(&head));
    assert_eq!(refs.get("HEAD"), Some(&head));
    Ok(())
}

pub async fn clone_non_empty_repo(pool: PgPool, transport: Transport, protocol: Protocol) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (_, head) = fixture.push_seed().await?;

    let clone = fixture.checkout(&fixture.owner, "clone").with_protocol(protocol);
    clone.clone_into(&fixture.owner_remote()).await?;

    assert_eq!(clone.rev_parse("HEAD").await?, head);
    assert_eq!(clone.ok(&["symbolic-ref", "HEAD"]).await?.trim(), "refs/heads/main");
    assert_eq!(clone.read_file("README.md").await?, "# test\n");
    clone.fsck().await?;
    Ok(())
}

pub async fn fetch_after_server_change(pool: PgPool, transport: Transport, protocol: Protocol) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (local, _) = fixture.push_seed().await?;

    let clone = fixture.checkout(&fixture.owner, "clone").with_protocol(protocol);
    clone.clone_into(&fixture.owner_remote()).await?;

    let head = local.commit_file("second.txt", "second\n", "second commit").await?;
    local.ok(&["push", "--quiet"]).await?;

    clone.ok(&["fetch", "--quiet"]).await?;

    assert_eq!(clone.rev_parse("origin/main").await?, head);
    clone.ok(&["merge", "--quiet", "--ff-only", "origin/main"]).await?;
    assert_eq!(clone.read_file("second.txt").await?, "second\n");
    Ok(())
}

pub async fn ls_remote_lists_all_refs(pool: PgPool, transport: Transport, protocol: Protocol) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (local, head) = fixture.push_seed().await?;

    local.ok(&["branch", "feature"]).await?;
    local.ok(&["tag", "v1"]).await?;
    local.ok(&["push", "--quiet", "origin", "feature", "v1"]).await?;

    let refs = fixture
        .owner
        .git(fixture.dir.path())
        .with_protocol(protocol)
        .ls_remote(&fixture.owner_remote())
        .await?;

    assert_eq!(
        refs.into_iter().collect::<Vec<_>>(),
        vec![
            ("HEAD".to_owned(), head.clone()),
            ("refs/heads/feature".to_owned(), head.clone()),
            ("refs/heads/main".to_owned(), head.clone()),
            ("refs/tags/v1".to_owned(), head),
        ]
    );
    Ok(())
}

pub async fn ls_remote_peels_annotated_tags(pool: PgPool, transport: Transport, protocol: Protocol) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (local, head) = fixture.push_seed().await?;

    local.ok(&["tag", "--annotate", "--message", "release", "v1"]).await?;
    local.ok(&["push", "--quiet", "origin", "v1"]).await?;

    let refs = fixture
        .owner
        .git(fixture.dir.path())
        .with_protocol(protocol)
        .ls_remote(&fixture.owner_remote())
        .await?;

    assert_eq!(refs.get("refs/tags/v1"), Some(&local.rev_parse("v1").await?));
    assert_eq!(refs.get("refs/tags/v1^{}"), Some(&head));
    Ok(())
}

pub async fn push_new_branch(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (local, _) = fixture.push_seed().await?;

    local.ok(&["switch", "--quiet", "--create", "feature"]).await?;
    let feature = local.commit_file("feature.txt", "feature\n", "feature commit").await?;
    local.ok(&["push", "--quiet", "origin", "feature"]).await?;

    assert_eq!(fixture.refs().await?.get("refs/heads/feature"), Some(&feature));
    Ok(())
}

pub async fn push_tags(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (local, head) = fixture.push_seed().await?;

    local.ok(&["tag", "lightweight"]).await?;
    local.ok(&["tag", "--annotate", "--message", "release", "annotated"]).await?;
    local.ok(&["push", "--quiet", "origin", "--tags"]).await?;

    let refs = fixture.refs().await?;
    assert_eq!(refs.get("refs/tags/lightweight"), Some(&head));
    assert_eq!(refs.get("refs/tags/annotated"), Some(&local.rev_parse("annotated").await?));

    let clone = fixture.checkout(&fixture.owner, "clone");
    clone.clone_into(&fixture.owner_remote()).await?;

    assert_eq!(clone.ok(&["cat-file", "-t", "annotated"]).await?.trim(), "tag");
    assert_eq!(clone.rev_parse("annotated^{commit}").await?, head);
    clone.fsck().await?;
    Ok(())
}

pub async fn force_push(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (local, base) = fixture.push_seed().await?;

    let pushed = local.commit_file("a.txt", "a\n", "pushed commit").await?;
    local.ok(&["push", "--quiet"]).await?;

    local.ok(&["reset", "--quiet", "--hard", &base]).await?;
    let rewritten = local.commit_file("b.txt", "b\n", "rewritten commit").await?;

    let stderr = local.fails(&["push", "origin", "main"]).await?;
    assert!(stderr.contains("[rejected]"), "{stderr}");
    assert_eq!(fixture.refs().await?.get("refs/heads/main"), Some(&pushed));

    local.ok(&["push", "--quiet", "--force", "origin", "main"]).await?;
    assert_eq!(fixture.refs().await?.get("refs/heads/main"), Some(&rewritten));
    Ok(())
}

pub async fn delete_branch(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (local, head) = fixture.push_seed().await?;

    local.ok(&["push", "--quiet", "origin", "main:refs/heads/feature"]).await?;
    assert_eq!(fixture.refs().await?.get("refs/heads/feature"), Some(&head));

    local.ok(&["push", "--quiet", "origin", "--delete", "feature"]).await?;

    let refs = fixture.refs().await?;
    assert_eq!(refs.get("refs/heads/feature"), None);
    assert_eq!(refs.get("refs/heads/main"), Some(&head));
    Ok(())
}

pub async fn delete_tag(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (local, head) = fixture.push_seed().await?;

    local.ok(&["tag", "v1"]).await?;
    local.ok(&["push", "--quiet", "origin", "v1"]).await?;
    assert_eq!(fixture.refs().await?.get("refs/tags/v1"), Some(&head));

    local.ok(&["push", "--quiet", "origin", "--delete", "refs/tags/v1"]).await?;

    assert_eq!(fixture.refs().await?.get("refs/tags/v1"), None);
    Ok(())
}

pub async fn delete_multiple_refs_in_one_push(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (local, head) = fixture.push_seed().await?;

    local.ok(&["tag", "v1"]).await?;
    local.ok(&["push", "--quiet", "origin", "main:refs/heads/feature", "v1"]).await?;

    local.ok(&["push", "--quiet", "origin", "--delete", "feature", "refs/tags/v1"]).await?;

    let refs = fixture.refs().await?;
    assert_eq!(refs.get("refs/heads/feature"), None);
    assert_eq!(refs.get("refs/tags/v1"), None);
    assert_eq!(refs.get("refs/heads/main"), Some(&head));
    Ok(())
}

pub async fn owner_can_clone_and_push_private_repo(pool: PgPool, transport: Transport, protocol: Protocol) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Private).await?;
    let (_, head) = fixture.push_seed().await?;

    let clone = fixture.checkout(&fixture.owner, "clone").with_protocol(protocol);
    clone.clone_into(&fixture.owner_remote()).await?;
    assert_eq!(clone.rev_parse("HEAD").await?, head);

    let next = clone.commit_file("next.txt", "next\n", "next commit").await?;
    clone.ok(&["push", "--quiet"]).await?;

    assert_eq!(fixture.refs().await?.get("refs/heads/main"), Some(&next));
    Ok(())
}

pub async fn other_user_can_clone_public_repo(pool: PgPool, transport: Transport, protocol: Protocol) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (_, head) = fixture.push_seed().await?;
    let bob = fixture.user("bob").await?;

    let clone = fixture.checkout(&bob, "clone").with_protocol(protocol);
    clone.clone_into(&fixture.remote(Some(&bob))).await?;

    assert_eq!(clone.rev_parse("HEAD").await?, head);
    Ok(())
}

pub async fn other_user_cannot_push_to_public_repo(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (_, head) = fixture.push_seed().await?;
    let bob = fixture.user("bob").await?;

    let clone = fixture.checkout(&bob, "clone");
    clone.clone_into(&fixture.remote(Some(&bob))).await?;
    clone.commit_file("bob.txt", "bob\n", "bob's commit").await?;

    clone.fails(&["push", "origin", "main"]).await?;
    assert_eq!(fixture.refs().await?.get("refs/heads/main"), Some(&head));
    Ok(())
}

pub async fn other_user_cannot_clone_private_repo(pool: PgPool, transport: Transport, protocol: Protocol) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Private).await?;
    fixture.push_seed().await?;
    let bob = fixture.user("bob").await?;

    let clone = fixture.checkout(&bob, "clone").with_protocol(protocol);
    clone.init().await?;
    clone.fails(&["fetch", &fixture.remote(Some(&bob)), "main"]).await?;
    Ok(())
}

pub async fn other_user_cannot_push_to_private_repo(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Private).await?;
    let (_, head) = fixture.push_seed().await?;
    let bob = fixture.user("bob").await?;

    let local = fixture.checkout(&bob, "local");
    local.seed().await?;

    local.fails(&["push", &fixture.remote(Some(&bob)), "main:refs/heads/bob"]).await?;

    let refs = fixture.refs().await?;
    assert_eq!(refs.get("refs/heads/bob"), None);
    assert_eq!(refs.get("refs/heads/main"), Some(&head));
    Ok(())
}

pub async fn nonexistent_repo_fails(pool: PgPool, transport: Transport, protocol: Protocol) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;

    let clone = fixture.checkout(&fixture.owner, "clone").with_protocol(protocol);
    clone.init().await?;

    clone
        .fails(&["fetch", &fixture.server.remote(transport, Some(&fixture.owner), "alice/missing")])
        .await?;
    clone
        .fails(&["fetch", &fixture.server.remote(transport, Some(&fixture.owner), "nobody/missing")])
        .await?;
    Ok(())
}

pub async fn nonexistent_repo_push_fails(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;

    let local = fixture.checkout(&fixture.owner, "local");
    local.seed().await?;

    local
        .fails(&["push", &fixture.server.remote(transport, Some(&fixture.owner), "alice/missing"), "main"])
        .await?;
    Ok(())
}

pub async fn push_with_secret_is_rejected(pool: PgPool, transport: Transport) -> Result<()> {
    let fixture = Fixture::new(&pool, transport, Visibility::Public).await?;
    let (local, head) = fixture.push_seed().await?;

    local.commit_file("config.env", &format!("TOKEN={SECRET}\n"), "add config").await?;

    let stderr = local.fails(&["push", "origin", "main"]).await?;

    assert!(stderr.contains("SECRETS DETECTED IN PUSH"), "{stderr}");
    assert!(stderr.contains("GitArena Personal Access Token"), "{stderr}");
    assert!(stderr.contains("push contains secrets"), "{stderr}");
    assert!(!stderr.contains(SECRET), "{stderr}");
    assert_eq!(fixture.refs().await?.get("refs/heads/main"), Some(&head));
    Ok(())
}
