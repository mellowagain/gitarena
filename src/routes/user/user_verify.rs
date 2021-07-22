use crate::error::GAErrors::HttpError;

use actix_web::{Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use log::info;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

#[route("/api/verify/{token}", method="GET")]
pub(crate) async fn verify(verify_request: web::Path<VerifyRequest>, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let token = &verify_request.token;

    if token.len() != 32 || !token.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(HttpError(400, "Token is illegal".to_owned()).into());
    }

    let mut transaction = db_pool.begin().await?;

    let option: Option<(i32, i32)> = sqlx::query_as("select id, user_id from user_verifications where hash = $1 and expires > now() limit 1")
        .bind(&token)
        .fetch_optional(&mut transaction)
        .await?;

    if option.is_none() {
        return Err(HttpError(403, "Token does not exist or has expired".to_owned()).into());
    }

    let (row_id, user_id) = option.unwrap();

    sqlx::query("update user_verifications set expires = now() - interval '1 day' where id = $1")
        .bind(&row_id)
        .execute(&mut transaction)
        .await?;

    transaction.commit().await?;

    info!("User id {} verified their e-mail", user_id);

    Ok(web::Json(json!({
        "success": true
    })))
}

#[derive(Deserialize)]
pub(crate) struct VerifyRequest {
    token: String
}
