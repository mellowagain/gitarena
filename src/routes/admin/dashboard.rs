use crate::prelude::ContextExtensions;
use crate::repository::Repository;
use crate::user::{User, WebUser};
use crate::{die, render_template};

use actix_web::{Responder, web};
use anyhow::Result;
use chrono::Duration;
use chrono_humanize::{Accuracy, HumanTime, Tense};
use git2::Version as LibGit2Version;
use gitarena_macros::route;
use heim::units::{Information, information, Time};
use once_cell::sync::Lazy;
use sqlx::PgPool;
use tera::Context;

#[route("/", method = "GET", err = "html")]
pub(crate) async fn dashboard(web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if !user.admin {
        die!(FORBIDDEN, "Not allowed");
    }

    let mut context = Context::new();

    // Users
    context.insert_user(&user)?;

    let mut transaction = db_pool.begin().await?;

    let (users_count,): (i64,) = sqlx::query_as("select count(*) from users")
        .fetch_one(&mut transaction)
        .await?;

    context.try_insert("users_count", &users_count)?;

    let latest_user_option: Option<User> = sqlx::query_as::<_, User>("select * from users order by id desc limit 1")
        .fetch_optional(&mut transaction)
        .await?;

    if let Some(latest_user) = latest_user_option {
        context.try_insert("latest_user", &latest_user)?;
    }

    // Groups

    context.try_insert("groups_count", &0_i32)?; // TODO: Update once groups are a thing

    // Repos

    let (repos_count,): (i64,) = sqlx::query_as("select count(*) from repositories")
        .fetch_one(&mut transaction)
        .await?;

    context.try_insert("repos_count", &repos_count)?;

    let latest_repo_option: Option<Repository> = sqlx::query_as::<_, Repository>("select * from repositories order by id desc limit 1")
        .fetch_optional(&mut transaction)
        .await?;

    if let Some(latest_repo) = latest_repo_option {
        context.try_insert("latest_repo", &latest_repo)?;

        let (latest_repo_username_option,): (String,) = sqlx::query_as("select username from users where id = $1 limit 1")
            .bind(&latest_repo.owner)
            .fetch_one(&mut transaction)
            .await?;

        context.try_insert("latest_repo_username", &latest_repo_username_option)?;
    }

    // Components
    context.try_insert("rustc_version", env!("VERGEN_RUSTC_SEMVER"))?;

    let (postgres_version,): (String,) = sqlx::query_as("show server_version")
        .fetch_one(&mut transaction)
        .await?;

    context.try_insert("postgres_version", postgres_version.as_str())?;

    const GITARENA_SHA1: &str = env!("VERGEN_GIT_SHA");
    static GITARENA_SHA1_SHORT: Lazy<&'static str> = Lazy::new(|| &GITARENA_SHA1[0..7]);
    static GITARENA_VERSION: Lazy<String> = Lazy::new(|| format!("{}-{}", env!("CARGO_PKG_VERSION"), *GITARENA_SHA1_SHORT));

    context.try_insert("gitarena_version", GITARENA_VERSION.as_str())?;

    let libgit2_version = LibGit2Version::get();
    let (major, minor, patch) = libgit2_version.libgit2_version();
    context.try_insert("libgit2_version", format!("{}.{}.{}", major, minor, patch).as_str())?;
    context.try_insert("git2_rs_version", libgit2_version.crate_version())?;

    // System Info

    if let Ok(platform) = heim::host::platform().await {
        context.try_insert("os", platform.system())?;
        context.try_insert("version", platform.release())?;
        context.try_insert("architecture", platform.architecture().as_str())?;
    }

    if let Ok(uptime) = heim::host::uptime().await {
        context.try_insert("uptime", format_heim_time(uptime).as_str())?;
    }

    if let Ok(memory) = heim::memory::memory().await {
        context.try_insert("memory_available", &heim_size_to_bytes(memory.available()))?;
        context.try_insert("memory_total", &heim_size_to_bytes(memory.total()))?;
    }

    if let Ok(process) = heim::process::current().await {
        context.try_insert("pid", &process.pid())?;
    }

    render_template!("admin/dashboard.html", context, transaction)
}

fn format_heim_time(time: Time) -> String {
    let duration = Duration::seconds(time.get::<heim::units::time::second>() as i64);
    let human_time = HumanTime::from(duration);

    human_time.to_text_en(Accuracy::Rough, Tense::Present)
}

fn heim_size_to_bytes(info: Information) -> usize {
    info.get::<information::byte>() as usize
}
