use crate::config::{get_setting, set_setting};
use crate::database::{Database, Pool};
use crate::git::hooks::index_zoekt::schedule_repo_indexing;
use crate::organization::Organization;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::user::User;
use anyhow::{Context, Error, Result};
use gitarena_macros::from_config;
use gix::config::parse::section::ValueName;
use gix::config::{File, Source};
use sqlx::Transaction;
use std::fs;
use std::path::PathBuf;
use tokio::task::spawn_blocking;
use tracing::{info, instrument};

pub(crate) mod task;

const ZOEKT_GITARENA_VERSION: i32 = 1;

pub(crate) async fn init(db_pool: &Pool) -> Result<()> {
    let enabled = from_config!("zoekt.enabled" => bool);

    if !enabled {
        info!("code searching unavailable as zoekt is disabled");
        return Ok(());
    }

    let mut tx = db_pool.begin().await?;

    maybe_reindex(&mut tx).await?;

    tx.commit().await?;
    Ok(())
}

#[instrument(err, skip(tx))]
pub(crate) async fn init_zoekt_config(repo: &Repository, tx: &mut Transaction<'_, Database>) -> Result<()> {
    let username = if let Some(user_id) = repo.owner_user {
        User::find_using_id(user_id, tx).await.expect("repo to be owned by a user").username
    } else if let Some(org_id) = repo.owner_org {
        Organization::find_by_id(org_id, tx).await.expect("repo to be owned by a org").name
    } else {
        unreachable!("database enforces one non-null on repo owner");
    };

    let domain: String = get_setting("domain", tx).await?;

    let mut path = PathBuf::from(repo.get_fs_path(tx).await?);
    path.push("config");

    let mut git_config = File::from_path_no_includes(path.clone(), Source::Local).context("failed to open git config")?;

    git_config.remove_section("zoekt", None);

    let mut section = git_config.new_section("zoekt", None).context("failed to create zoekt section")?;

    section.push(ValueName::try_from("repoid")?, Some(repo.zoekt_id.to_string().as_str().into()));
    section.push(ValueName::try_from("name")?, Some(format!("{username}/{}", repo.name).as_str().into()));
    section.push(
        ValueName::try_from("web-url")?,
        Some(format!("{domain}/{username}/{}", repo.name).as_str().into()),
    );
    section.push(ValueName::try_from("web-url-type")?, Some("gitea".into())); // seems compatible with our url schema

    section.push(ValueName::try_from("archived")?, Some(repo.archived_at.is_some().to_string().as_str().into()));
    section.push(ValueName::try_from("fork")?, Some(repo.forked_from.is_some().to_string().as_str().into()));
    section.push(
        ValueName::try_from("public")?,
        Some(matches!(repo.visibility, RepoVisibility::Public).to_string().as_str().into()),
    );

    spawn_blocking(move || {
        let mut file = fs::File::create(path).context("failed to open git config in rw mode")?;
        git_config.write_to(&mut file).context("failed to write git config")?;
        Ok::<_, Error>(())
    })
    .await??;

    Ok(())
}

#[instrument(err, skip(tx))]
pub(crate) async fn maybe_reindex(tx: &mut Transaction<'_, Database>) -> Result<()> {
    let version: i32 = get_setting("zoekt.gitarena_version", tx).await?;

    if version >= ZOEKT_GITARENA_VERSION {
        return Ok(());
    }

    info!(
        found = version,
        expected = ZOEKT_GITARENA_VERSION,
        "zoekt gitarena version out of date. re-indexing all repos..."
    );

    let repos = sqlx::query_as::<_, Repository>("select * from repositories").fetch_all(&mut **tx).await?;

    for repo in repos {
        init_zoekt_config(&repo, tx).await?;
        schedule_repo_indexing(repo, tx).await?;
    }

    set_setting("zoekt.gitarena_version", ZOEKT_GITARENA_VERSION, tx).await?;
    Ok(())
}
