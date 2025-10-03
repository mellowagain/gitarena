use crate::config::get_optional_setting;
use crate::die;
use crate::git::write;
use crate::prelude::HttpRequestExtensions;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::routes::repository::api::CreateJsonResponse;
use crate::user::{User, WebUser};
use crate::utils::identifiers::{is_fs_legal, is_reserved_repo_name, is_valid};

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use gitarena_macros::route;
use log::info;
use serde::Deserialize;
use sqlx::{PgPool, Pool, Postgres};

// This whole handler is very similar to `import_repo.rs` so at some point this should be consolidated into one

#[route("/api/repo", method = "POST", err = "json")]
pub(crate) async fn create(
    web_user: WebUser,
    body: web::Json<CreateJsonRequest>,
    request: HttpRequest,
    db_pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let user = web_user.into_user()?;

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

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from repositories where owner = $1 and lower(name) = lower($2) limit 1)")
        .bind(&user.id)
        .bind(&name)
        .fetch_one(&mut transaction)
        .await?;

    if exists {
        die!(CONFLICT, "Repository name already in use for your account");
    }

    let repo: Repository = sqlx::query_as::<_, Repository>("insert into repositories (owner, name, description, visibility) values ($1, $2, $3, $4) returning *")
        .bind(&user.id)
        .bind(name)
        .bind(description)
        .bind(&body.visibility)
        .fetch_one(&mut transaction)
        .await?;

    repo.create_fs(&mut transaction).await?;

    // Can be simplified once let chains are implemented: https://github.com/rust-lang/rust/issues/53667
    if body.readme.is_some() {
        create_readme(&repo, &user, &db_pool).await?;
    }

    let domain = get_optional_setting::<String, _>("domain", &mut transaction)
        .await?
        .unwrap_or_default();
    let path = format!("/{}/{}", &user.username, &repo.name);

    transaction.commit().await?;

    info!(
        "New repository created: {}/{} (id {})",
        &user.username, &repo.name, &repo.id
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

async fn create_readme(repo: &Repository, user: &User, db_pool: &Pool<Postgres>) -> Result<()> {
    let mut transaction = db_pool.begin().await?;
    let libgit2_repo = repo.libgit2(&mut transaction).await?;
    let readme = format!(
        "# {}\n\n{}\n",
        repo.name.as_str(),
        repo.description.as_str()
    );

    transaction.commit().await?;

    write::write_file(
        &libgit2_repo,
        user,
        Some("HEAD"),
        "README.md",
        readme.as_bytes(),
        db_pool,
    )
    .await
}

#[derive(Deserialize)]
pub(crate) struct CreateJsonRequest {
    name: String,
    description: String,
    visibility: RepoVisibility,
    #[serde(default)]
    readme: Option<String>,
}
