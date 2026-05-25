use crate::database::{Database, Pool};
use crate::issue::IssueCache;
use crate::repository::Repository;
use crate::user::User;
use anyhow::{Context, Result};
use gitarena_macros::from_config;
use meilisearch_sdk::client::Client;
use sqlx::Transaction;
use std::time::Duration;
use tracing::{error, info};

pub(crate) static REPOS_MEILI_INDEX: &str = "repositories";
pub(crate) static USERS_MEILI_INDEX: &str = "users";
pub(crate) static ISSUES_MEILI_INDEX: &str = "issues";

pub(crate) type MeiliClient = Option<Client>;

pub(crate) async fn init(db_pool: &Pool) -> Result<Option<Client>> {
    let (enabled, url, key) = from_config!(
        "meilisearch.enabled" => bool,
        "meilisearch.url" => String,
        "meilisearch.key" => String,
    );

    if !enabled {
        info!("general searching unavailable as meilisearch is disabled");
        return Ok(None);
    }

    let mut tx = db_pool.begin().await?;

    let client = Client::new(url, if key.is_empty() { None } else { Some(key) }).context("failed to initialize meilisearch client")?;

    reindex_repos(&client, &mut tx).await?;
    reindex_users(&client, &mut tx).await?;
    reindex_issues(&client, &mut tx).await?;

    tx.commit().await?;

    Ok(Some(client))
}

async fn reindex_repos(client: &Client, tx: &mut Transaction<'_, Database>) -> Result<()> {
    let repos = sqlx::query_as::<_, Repository>("select * from repositories").fetch_all(&mut **tx).await?;

    client
        .index(REPOS_MEILI_INDEX)
        .set_filterable_attributes(&["visibility", "disabled", "owner_user", "owner_org", "id"])
        .await
        .context("failed to set filterable attributes on repos meilisearch index")?;

    let task = client
        .index(REPOS_MEILI_INDEX)
        .add_documents(&repos, Some("id"))
        .await
        .context("failed to enqueue indexing task all repos to meilisearch")?;

    let cloned_client = client.clone();

    tokio::spawn(async move {
        match task.wait_for_completion(&cloned_client, None, Some(Duration::from_mins(5))).await {
            Ok(_) => info!("successfully re-indexed all repos into meilisearch"),
            Err(err) => error!(?err, "failed to re-index all repos into meilisearch"),
        }
    });

    Ok(())
}

async fn reindex_users(client: &Client, tx: &mut Transaction<'_, Database>) -> Result<()> {
    let users = sqlx::query_as::<_, User>("select * from users").fetch_all(&mut **tx).await?;

    client
        .index(USERS_MEILI_INDEX)
        .set_filterable_attributes(&["disabled"])
        .await
        .context("failed to set filterable attributes on users meilisearch index")?;

    let task = client
        .index(USERS_MEILI_INDEX)
        .add_documents(&users, Some("id"))
        .await
        .context("failed to enqueue indexing task all users to meilisearch")?;

    let cloned_client = client.clone();

    tokio::spawn(async move {
        match task.wait_for_completion(&cloned_client, None, Some(Duration::from_mins(5))).await {
            Ok(_) => info!("successfully re-indexed all users into meilisearch"),
            Err(err) => error!(?err, "failed to re-index all users into meilisearch"),
        }
    });

    Ok(())
}

async fn reindex_issues(client: &Client, tx: &mut Transaction<'_, Database>) -> Result<()> {
    let issues = sqlx::query_as::<_, IssueCache>("select * from issue_cache").fetch_all(&mut **tx).await?;

    client
        .index(ISSUES_MEILI_INDEX)
        .set_filterable_attributes(&["repo_id", "open", "confidential", "priority"])
        .await
        .context("failed to set filterable attributes on issues meilisearch index")?;

    let task = client
        .index(ISSUES_MEILI_INDEX)
        .add_documents(&issues, Some("id"))
        .await
        .context("failed to enqueue indexing task all issues to meilisearch")?;

    let cloned_client = client.clone();

    tokio::spawn(async move {
        match task.wait_for_completion(&cloned_client, None, Some(Duration::from_mins(5))).await {
            Ok(_) => info!("successfully re-indexed all issues into meilisearch"),
            Err(err) => error!(?err, "failed to re-index all issues into meilisearch"),
        }
    });

    Ok(())
}
