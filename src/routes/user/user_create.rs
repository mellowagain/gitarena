use crate::error::GAErrors::HttpError;
use crate::user::User;
use crate::verification::send_verification_mail;
use crate::{captcha, GaE, crypto, extensions};

use actix_session::Session;
use actix_web::{HttpResponse, post, Responder, Result as ActixResult, web, HttpRequest};
use anyhow::{Context, Result};
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// POST /api/users
async fn register(session: Session, body: web::Json<RegisterJsonRequest>, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let username = &body.username;

    if username.len() < 3 || username.len() > 32 || !username.chars().all(|c| is_username(&c)) {
        return Err(HttpError(400, "Username must be between 3 and 32 characters long and may only contain a-z, 0-9, _ or -".to_owned()).into());
    }

    let lowered_username = username.to_lowercase();

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from users where lower(username) = $1);")
        .bind(&lowered_username)
        .fetch_one(&mut transaction)
        .await?;

    if exists {
        return Err(HttpError(409, "Username already in use".to_owned()).into());
    }

    let email = &body.email;

    // This is not according to the spec of the IETF but trying to implement that is honestly out-of-bounds for this project
    // Thus a best effort naive implementation. Checks for the presence of "@" and a "." in the domain name (after the last @)
    if !email.contains("@") || !email.rsplitn(2, "@").next().unwrap_or_default().contains(".") {
        return Err(HttpError(400, "Invalid email address".to_owned()).into());
    }

    let raw_password = &body.password;

    // We don't implement any strict password rules according to NIST 2017 Guidelines
    if raw_password.len() < 8 {
        return Err(HttpError(400, "Password must be at least 8 characters".to_owned()).into());
    }

    let password = crypto::hash_password(raw_password)?;

    let captcha_success = captcha::verify_captcha(&body.h_captcha_response.to_owned()).await?;

    if !captcha_success {
        return Err(HttpError(422, "Captcha verification failed".to_owned()).into());
    }

    let user: User = sqlx::query_as::<_, User>("insert into users (username, email, password) values ($1, $2, $3) returning *")
        .bind(username)
        .bind(email)
        .bind(&password)
        .fetch_one(&mut transaction)
        .await?;

    send_verification_mail(&user, &mut transaction).await?;

    let user_agent = extensions::get_user_agent(&request).unwrap_or_default()
        .chars()
        .take(256)
        .collect::<String>();

    let (hash,): (String,) = sqlx::query_as("insert into user_sessions (user_id, user_agent) values ($1, $2) returning session")
        .bind(&user.id)
        .bind(user_agent)
        .fetch_one(&mut transaction)
        .await
        .context("Failed to create user session")?;

    if session.set("user_session", hash).is_err() {
        return Err(HttpError(500, "Failed to set user session".to_owned()).into());
    }

    transaction.commit().await?;

    info!("New user registered: {} (id {})", &user.username, &user.id);

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
pub(crate) async fn handle_post(session: Session, body: web::Json<RegisterJsonRequest>, request: HttpRequest, db_pool: web::Data<PgPool>) -> ActixResult<impl Responder> {
    Ok(register(session, body, request, db_pool).await.map_err(|e| -> GaE { e.into() }))
}
