use crate::die;
use gitarena_common::database::Pool;

use actix_web::http::header::LOCATION;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde::Deserialize;
use tracing::info;
use tracing_unwrap::OptionExt;
use uuid::Uuid;

#[route("/api/verify/{token}", method = "GET", err = "json")]
pub(crate) async fn verify(verify_request: web::Path<VerifyRequest>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let token = &verify_request.token;

    if token.len() != 32 || !token.chars().all(|c| c.is_ascii_hexdigit()) {
        die!(BAD_REQUEST, "Token is illegal");
    }

    let mut transaction = db_pool.begin().await?;

    let option: Option<(Uuid, Uuid)> = sqlx::query_as("select id, user_id from user_verifications where hash = $1 and expires > now() limit 1")
        .bind(token)
        .fetch_optional(&mut *transaction)
        .await?;

    if option.is_none() {
        die!(FORBIDDEN, "Token does not exist or has expired");
    }

    let (row_id, user_id) = option.unwrap_or_log();

    sqlx::query("update emails set verified_at = current_timestamp where owner = $1")
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;

    sqlx::query("delete from user_verifications where id = $1")
        .bind(row_id)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;

    info!(user.id = %user_id, "User verified their e-mail");

    Ok(HttpResponse::TemporaryRedirect().append_header((LOCATION, "/?verified=true")).finish())
}

#[derive(Deserialize)]
pub(crate) struct VerifyRequest {
    token: String,
}
