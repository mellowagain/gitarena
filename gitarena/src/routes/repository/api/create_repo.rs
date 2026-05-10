use crate::config::get_optional_setting;
use crate::die;
use crate::git::write;
use crate::organization::{OrgMember, Organization};
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::routes::repository::api::CreateJsonResponse;
use crate::user::WebUser;
use crate::utils::identifiers::{is_fs_legal, is_reserved_repo_name, is_valid};

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_common::database::Pool;
use gitarena_macros::route;
use serde::Deserialize;
use tracing::{info, instrument};
use utoipa::ToSchema;
use uuid::Uuid;

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

    let mut tx = db_pool.begin().await?;

    // Resolve namespace to either the authenticated user or an org the user has access to
    let (owner_id, owner_name) = if body.namespace == user.username {
        (user.id, user.username.clone())
    } else if let Some(org) = Organization::find_by_name(&body.namespace, &mut tx).await {
        // User must be a member of the org to create repositories under it
        if OrgMember::get_role(org.id, user.id, &mut tx).await?.is_none() {
            die!(FORBIDDEN, "You are not a member of this organization");
        }
        (org.id, org.name.clone())
    } else {
        die!(BAD_REQUEST, "Namespace not found or you do not have access to it");
    };

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from repositories where owner = $1 and lower(name) = lower($2) limit 1)")
        .bind(owner_id)
        .bind(name)
        .fetch_one(&mut *tx)
        .await?;

    if exists {
        die!(CONFLICT, "Repository name already in use for this namespace");
    }

    let repo: Repository = sqlx::query_as::<_, Repository>(
        "insert into repositories (id, owner, name, description, visibility, default_branch) values ($1, $2, $3, $4, $5, $6) returning *",
    )
    .bind(Uuid::now_v7())
    .bind(owner_id)
    .bind(name)
    .bind(description)
    .bind(body.visibility)
    .bind(&body.default_branch)
    .fetch_one(&mut *tx)
    .await?;

    repo.create_fs(&mut tx).await?;

    // Can be simplified once let chains are implemented: https://github.com/rust-lang/rust/issues/53667
    if let Some(readme) = body.readme
        && readme
    {
        let readme = format!("# {}\n\n{}\n", repo.name.as_str(), repo.description.as_str());
        create_file(&repo, &user, "README.md", &readme, &db_pool).await?;
    }

    if let Some(_license) = &body.license {
        // todo: include https://github.com/github/choosealicense.com/tree/gh-pages/_licenses
    }

    if let Some(_gitignore) = &body.gitignore {
        // todo: include https://github.com/github/gitignore
    }

    let domain = get_optional_setting::<String>("domain", &mut tx).await?.unwrap_or_default();
    let path = format!("/{}/{}", &owner_name, &repo.name);

    tx.commit().await?;

    info!(id = %repo.id, owner = owner_name, name = repo.name.as_str(), "New repository created");

    Ok(HttpResponse::Ok().json(CreateJsonResponse {
        id: repo.id,
        url: format!("{domain}{path}"),
    }))
}

#[instrument(err, skip(db_pool))]
async fn create_file(repo: &Repository, user: &crate::user::User, file_name: &str, content: &str, db_pool: &Pool) -> Result<()> {
    let libgit2_repo = {
        let mut transaction = db_pool.begin().await?;
        let libgit2_repo = repo.libgit2(&mut transaction).await?;
        transaction.commit().await?;
        libgit2_repo
    };

    write::write_file(&libgit2_repo, user, Some("HEAD"), file_name, content.as_bytes(), db_pool).await
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateJsonRequest {
    /// Namespace (username or org name) under which the repo should be created
    namespace: String,
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
    readme: Option<bool>,
    /// Default branch for repository
    default_branch: String,
    /// Optional: initialioze a LICENSE.txt file with the chosen license
    #[serde(default)]
    license: Option<String>,
    /// Optional: initialize a .gitignore file with the chosen template
    #[serde(default)]
    gitignore: Option<String>,
}
