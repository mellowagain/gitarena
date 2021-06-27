use crate::error::GAErrors::HttpError;
use crate::user::User;
use crate::verification::send_verification_mail;
use crate::{captcha, GaE, templates};

use actix_web::{HttpResponse, post, Responder, Result as ActixResult, web};
use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// POST /api/users
async fn register(body: web::Json<RegisterJsonRequest>, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let username = &body.username;

    if username.len() < 3 || !username.chars().all(|c| is_username(&c)) {
        return Err(HttpError(400, "Username must be at least 3 characters and may only contain a-z, 0-9, _ or -".to_owned()).into());
    }

    let lowered_username = username.to_lowercase();

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from users where lower(username) = $1);")
        .bind(&lowered_username)
        .fetch_one(&mut transaction)
        .await?;

    if exists {
        return Err(HttpError(409, "Username already in use".to_owned()).into());
    }

    let captcha_success = captcha::verify_captcha(&body.h_captcha_response.to_owned()).await?;

    if !captcha_success {
        return Err(HttpError(422, "Captcha verification failed".to_owned()).into());
    }

    let mut user = User::new(
        username.to_owned(), body.email.to_owned(), body.password.to_owned()
    )?;
    user.save(&mut transaction).await?;

    send_verification_mail(&user, &mut transaction).await?;

    transaction.commit().await?;

    info!("New user registered: {} (id {})", user.username, user.id);

    Ok(HttpResponse::Ok().json(RegisterJsonResponse {
        success: true,
        id: user.id
    }).await)
}

#[inline]
fn is_username(c: &char) -> bool {
    c.is_ascii_alphanumeric() || c == &'-' || c == &'_'
}

#[derive(Deserialize)]
pub(crate) struct RegisterJsonRequest {
    username: String,
    email: String,
    password: String,
    h_captcha_response: String
}

#[derive(Serialize)]
struct RegisterJsonResponse {
    success: bool,
    id: i32
}

#[post("/api/user")]
pub(crate) async fn handle_post(body: web::Json<RegisterJsonRequest>, db_pool: web::Data<PgPool>) -> ActixResult<impl Responder> {
    Ok(register(body, db_pool).await.map_err(|e| -> GaE { e.into() }))
}
