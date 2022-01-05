use crate::config::get_optional_setting;
use crate::die;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::user::WebUser;
use crate::utils::identifiers::{is_fs_legal, is_reserved_repo_name, is_valid};

use actix_web::{HttpResponse, Responder, web};
use sqlx::PgPool;
use anyhow::Result;
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use log::info;

#[route("/api/repo", method = "POST", err = "json")]
pub(crate) async fn create(web_user: WebUser, body: web::Json<CreateJsonRequest>, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
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
        die!(BAD_REQUEST, "Description may only be up to 256 characters long");
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

    let domain = get_optional_setting::<String, _>("domain", &mut transaction).await?.unwrap_or_default();
    let url = format!("{}/{}/{}", domain, &user.username, &repo.name);

    transaction.commit().await?;

    info!("New repository created: {}/{} (id {})", &user.username, &repo.name, &repo.id);

    Ok(HttpResponse::Ok().json(CreateJsonResponse {
        id: repo.id,
        url
    }))
}

#[derive(Deserialize)]
pub(crate) struct CreateJsonRequest {
    name: String,
    description: String,
    visibility: RepoVisibility
}

#[derive(Serialize)]
pub(crate) struct CreateJsonResponse {
    id: i32,
    url: String
}
