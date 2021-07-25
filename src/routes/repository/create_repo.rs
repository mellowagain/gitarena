use crate::config::CONFIG;
use crate::error::GAErrors::HttpError;
use crate::extensions::{get_user_by_identity, is_fs_legal, is_identifier};
use crate::repository::Repository;

use std::borrow::Borrow;

use actix_web::{HttpResponse, Responder, web};
use sqlx::PgPool;
use actix_identity::Identity;
use anyhow::Result;
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use log::info;

#[route("/api/repo", method="POST")]
pub(crate) async fn create(id: Identity, body: web::Json<CreateJsonRequest>, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let user = match get_user_by_identity(id.identity(), &mut transaction).await {
        Some(user) => user,
        None => return Err(HttpError(401, "Not logged in".to_owned()).into())
    };

    let name = &body.name;

    if name.is_empty() || name.len() > 32 || !name.chars().all(|c| is_identifier(&c)) {
        return Err(HttpError(400, "Repository name must be between 1 and 32 characters long and may only contain a-z, 0-9, _ or -".to_owned()).into());
    }

    if !is_fs_legal(name).await {
        return Err(HttpError(400, "Repository name is illegal".to_owned()).into());
    }

    let description = &body.description;

    if description.len() > 256 {
        return Err(HttpError(400, "Description may only be up to 256 characters long".to_owned()).into());
    }

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from repositories where owner = $1 and lower(name) = lower($2));")
        .bind(&user.id)
        .bind(&name)
        .fetch_one(&mut transaction)
        .await?;

    if exists {
        return Err(HttpError(409, "Repository name already in use for your account".to_owned()).into());
    }

    let repo: Repository = sqlx::query_as::<_, Repository>("insert into repositories (owner, name, description, private) values ($1, $2, $3, $4) returning *")
        .bind(&user.id)
        .bind(name)
        .bind(description)
        .bind(&body.private)
        .fetch_one(&mut transaction)
        .await?;

    repo.create_fs(&user.username).await?;

    let domain: &str = CONFIG.domain.borrow();
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
    private: bool
}

#[derive(Serialize)]
pub(crate) struct CreateJsonResponse {
    id: i32,
    url: String
}
