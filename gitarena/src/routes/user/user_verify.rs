use crate::database::Pool;
use crate::die;

use crate::contributions::task::BackfillEmailContributionsTask;
use crate::events::{Event, Subject};
use crate::queue::GLOBAL_QUEUE;
use crate::user::WebUser;
use actix_web::http::header::LOCATION;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::{Result, anyhow};
use fang::{AsyncQueueable, AsyncRunnable};
use gitarena_macros::route;
use serde::Deserialize;
use serde_json::json;
use tracing::info;
use tracing_unwrap::OptionExt;
use uuid::Uuid;

#[route("/api/verify/{token}", method = "GET", err = "json")]
pub(crate) async fn verify(
    verify_request: web::Path<VerifyRequest>,
    web_user: WebUser,
    request: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let token = &verify_request.token;

    if token.len() != 32 || !token.chars().all(|c| c.is_ascii_hexdigit()) {
        die!(BAD_REQUEST, "Token is illegal");
    }

    let mut transaction = db_pool.begin().await?;

    let option: Option<(Uuid, Uuid, String)> = sqlx::query_as("select id, user_id, email from user_verifications where hash = $1 and expires > now() limit 1")
        .bind(token)
        .fetch_optional(&mut *transaction)
        .await?;

    if option.is_none() {
        die!(FORBIDDEN, "Token does not exist or has expired");
    }

    let (row_id, user_id, email) = option.unwrap_or_log();

    sqlx::query("update emails set verified_at = current_timestamp where owner = $1 and email = $2")
        .bind(user_id)
        .bind(&email)
        .execute(&mut *transaction)
        .await?;

    sqlx::query("delete from user_verifications where id = $1")
        .bind(row_id)
        .execute(&mut *transaction)
        .await?;

    Event::new(
        "email.verified",
        web_user.as_ref().map_or_else(|| Uuid::nil(), |user| user.id),
        &request,
        Subject::User(user_id),
        Some(json!({
            "email": email,
        })),
    )
    .save(&mut transaction)
    .await?;

    transaction.commit().await?;

    let task = BackfillEmailContributionsTask { email, user_id };

    GLOBAL_QUEUE
        .get()
        .ok_or_else(|| anyhow!("contributions backfill should only be scheduled after queue has been initialized"))?
        .insert_task(&task as &dyn AsyncRunnable)
        .await?;

    info!(user.id = %user_id, "User verified their e-mail");

    Ok(HttpResponse::TemporaryRedirect().append_header((LOCATION, "/?verified=true")).finish())
}

#[derive(Deserialize)]
pub(crate) struct VerifyRequest {
    token: String,
}
