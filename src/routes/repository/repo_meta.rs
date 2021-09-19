use crate::error::GAErrors::HttpError;
use crate::extensions::get_user_by_identity;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;

use actix_identity::Identity;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use sqlx::PgPool;

#[route("/api/repo/{username}/{repository}", method="GET")]
pub(crate) async fn meta(uri: web::Path<GitRequest>, id: Identity, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let (user_id,): (i32,) = sqlx::query_as("select id from users where lower(username) = lower($1)")
        .bind(&uri.username)
        .fetch_optional(&mut transaction)
        .await?
        .ok_or(HttpError(404, "Not found".to_owned()))?;

    let repo: Repository = sqlx::query_as::<_, Repository>("select * from repositories where owner = $1 and lower(name) = lower($2)")
        .bind(&user_id)
        .bind(&uri.repository)
        .fetch_optional(&mut transaction)
        .await?
        .ok_or(HttpError(404, "Not found".to_owned()))?;

    let user = get_user_by_identity(id.identity(), &mut transaction).await;

    if !privilege::check_access(&repo, &user, &mut transaction).await? {
        return Err(HttpError(404, "Not found".to_owned()).into());
    }

    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(repo))
}
