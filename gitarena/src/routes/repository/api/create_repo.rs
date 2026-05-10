use crate::config::get_optional_setting;
use crate::die;
use crate::git::write;
use crate::prelude::HttpRequestExtensions;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::routes::repository::api::CreateJsonResponse;
use crate::user::{User, WebUser};
use crate::utils::identifiers::{is_fs_legal, is_reserved_repo_name, is_valid};

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_common::database::Pool;
use gitarena_macros::route;
use serde::Deserialize;
use tracing::{info, instrument};
use utoipa::ToSchema;
use uuid::Uuid;

// This whole handler is very similar to `import_repo.rs` so at some point this should be consolidated into one

#[utoipa::path(
    post,
    path = "/api/repo",
    request_body = CreateJsonRequest,
    responses(
        (status = 200, description = "Repository created successfully", body = CreateJsonResponse),
        (status = 400, description = "Invalid repository name or description"),
        (status = 401, description = "Authentication required"),
        (status = 409, description = "Repository name already in use"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repo", method = "POST", err = "json")]
pub(crate) async fn create(web_user: WebUser, body: web::Json<CreateJsonRequest>, request: HttpRequest, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let user = web_user.into_user()?;

    let name = &body.name;

    if name.is_empty() || name.len() > 32 || !name.chars().all(is_valid) {
        die!(
            BAD_REQUEST,
            "Repository name must be between 1 and 32 characters long and may only contain a-z, 0-9, _ or -"
        );
    }

    if is_reserved_repo_name(name.as_str()) {
        die!(BAD_REQUEST, "Repository name is a reserved identifier");
    }

    if !is_fs_legal(name) {
        die!(BAD_REQUEST, "Repository name is illegal");
    }

    let description = &body.description;

    if description.len() > 256 {
        die!(BAD_REQUEST, "Description may only be up to 256 characters long");
    }

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from repositories where owner = $1 and lower(name) = lower($2) limit 1)")
        .bind(user.id)
        .bind(name)
        .fetch_one(&mut *transaction)
        .await?;

    if exists {
        die!(CONFLICT, "Repository name already in use for your account");
    }

    let repo: Repository =
        sqlx::query_as::<_, Repository>("insert into repositories (id, owner, name, description, visibility) values ($1, $2, $3, $4, $5) returning *")
            .bind(Uuid::now_v7())
            .bind(user.id)
            .bind(name)
            .bind(description)
            .bind(body.visibility)
            .fetch_one(&mut *transaction)
            .await?;

    repo.create_fs(&mut transaction).await?;

    // Can be simplified once let chains are implemented: https://github.com/rust-lang/rust/issues/53667
    if body.readme.is_some() {
        create_readme(&repo, &user, &db_pool).await?;
    }

    let domain = get_optional_setting::<String>("domain", &mut transaction).await?.unwrap_or_default();
    let path = format!("/{}/{}", &user.username, &repo.name);

    transaction.commit().await?;

    let owner = &user.username;
    let name = &repo.name;

    info!(id = %repo.id, owner, name, "New repository created");

    Ok(if request.is_htmx() {
        HttpResponse::Ok()
            .append_header(("hx-redirect", path))
            .append_header(("hx-refresh", "true"))
            .finish()
    } else {
        HttpResponse::Ok().json(CreateJsonResponse {
            id: repo.id,
            url: format!("{domain}{path}"),
        })
    })
}

#[instrument(err, skip(db_pool))]
async fn create_readme(repo: &Repository, user: &User, db_pool: &Pool) -> Result<()> {
    let mut transaction = db_pool.begin().await?;
    let libgit2_repo = repo.libgit2(&mut transaction).await?;
    let readme = format!("# {}\n\n{}\n", repo.name.as_str(), repo.description.as_str());

    transaction.commit().await?;

    write::write_file(&libgit2_repo, user, Some("HEAD"), "README.md", readme.as_bytes(), db_pool).await
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct CreateJsonRequest {
    /// Repository name
    #[schema(max_length = 32, pattern = "^[a-z0-9_-]+$")]
    name: String,
    /// Short description of the repository
    #[schema(max_length = 256)]
    description: String,
    /// Visibility of the repository
    visibility: RepoVisibility,
    /// Optional: initialise a README.md with the repo name and description
    #[serde(default)]
    readme: Option<String>,
}
