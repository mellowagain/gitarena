use crate::config::{get_optional_setting, get_setting};
use crate::prelude::HttpRequestExtensions;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::routes::repository::api::CreateJsonResponse;
use crate::user::WebUser;
use crate::utils::identifiers::{is_fs_legal, is_reserved_repo_name, is_valid};
use crate::{die, err, Ipc};

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use anyhow::{Context, Result};
use futures_locks::RwLock;
use gitarena_common::packets::git::GitImport;
use gitarena_macros::route;
use log::info;
use serde::Deserialize;
use sqlx::PgPool;
use url::Url;

// This whole handler is very similar to `create_repo.rs` so at some point this should be consolidated into one

#[route("/api/repo/import", method = "POST", err = "json")]
pub(crate) async fn import(
    web_user: WebUser,
    body: web::Json<ImportJsonRequest>,
    request: HttpRequest,
    ipc: web::Data<RwLock<Ipc>>,
    db_pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let mut transaction = db_pool.begin().await?;

    let enabled =
        get_setting::<bool, _>("repositories.importing_enabled", &mut transaction).await?;

    if !enabled || !ipc.read().await.is_connected() {
        die!(NOT_IMPLEMENTED, "Importing is disabled on this instance");
    }

    let name = &body.name;

    if name.is_empty() || name.len() > 32 || !name.chars().all(|c| is_valid(&c)) {
        die!(BAD_REQUEST, "Repository name must be between 1 and 32 characters long and may only contain a-z, 0-9, _ or -");
    }

    if is_reserved_repo_name(name.as_str()) {
        die!(BAD_REQUEST, "Repository name is a reserved identifier");
    }

    if !is_fs_legal(name) {
        die!(BAD_REQUEST, "Repository name is illegal");
    }

    let description = &body.description;

    if description.len() > 256 {
        die!(
            BAD_REQUEST,
            "Description may only be up to 256 characters long"
        );
    }

    let url = Url::parse(body.import_url.as_str())
        .map_err(|_| err!(BAD_REQUEST, "Unable to parse import url"))?;

    if body.mirror.is_some() {
        die!(NOT_IMPLEMENTED, "Mirroring is not yet implemented");
    }

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from repositories where owner = $1 and lower(name) = lower($2) limit 1)")
        .bind(user.id)
        .bind(name)
        .fetch_one(&mut transaction)
        .await?;

    if exists {
        die!(CONFLICT, "Repository name already in use for your account");
    }

    let repo: Repository = sqlx::query_as::<_, Repository>("insert into repositories (owner, name, description, visibility) values ($1, $2, $3, $4) returning *")
        .bind(user.id)
        .bind(name)
        .bind(description)
        .bind(&body.visibility)
        .fetch_one(&mut transaction)
        .await?;

    repo.create_fs(&mut transaction).await?;

    // Currently, only Git importing is supported. TODO: Support other VCS as well as GitLab export
    // At some point it is also planned to import issues and such, requiring support for specific hosters such as GitHub, GitLab, BitBucket and Gitea
    let packet = GitImport {
        url: url.to_string(),
        username: body.username.clone(),
        password: body.password.clone(),
    };

    ipc.write()
        .await
        .send(packet)
        .await
        .context("Failed to send import packet to workhorse")?;

    let domain = get_optional_setting::<String, _>("domain", &mut transaction)
        .await?
        .unwrap_or_default();
    let path = format!("/{}/{}", &user.username, &repo.name);

    transaction.commit().await?;

    info!(
        "New repository created for importing: {}/{} (id {}) (source: {})",
        &user.username, &repo.name, &repo.id, url
    );

    Ok(if request.is_htmx() {
        HttpResponse::Ok()
            .append_header(("hx-redirect", path))
            .append_header(("hx-refresh", "true"))
            .finish()
    } else {
        let url = format!("{}{}", domain, path);

        HttpResponse::Ok().json(CreateJsonResponse { id: repo.id, url })
    })
}

#[derive(Deserialize)]
pub(crate) struct ImportJsonRequest {
    //owner: String,
    name: String,
    description: String,
    #[serde(rename = "url")]
    import_url: String,
    #[serde(default)]
    mirror: Option<String>,
    visibility: RepoVisibility,

    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
}
