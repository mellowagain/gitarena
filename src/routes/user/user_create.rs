use crate::config::{get_optional_setting, get_setting};
use crate::error::GAErrors::HttpError;
use crate::prelude::*;
use crate::session::Session;
use crate::user::{User, WebUser};
use crate::utils::identifiers::{is_fs_legal, is_reserved_username, is_valid};
use crate::verification::send_verification_mail;
use crate::{captcha, crypto, render_template};

use actix_identity::Identity;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tera::Context;

#[route("/register", method = "GET")]
pub(crate) async fn get_register(web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    if matches!(web_user, WebUser::Authenticated(_)) {
        return Err(HttpError(401, "Already logged in".to_owned()).into());
    }

    let mut context = Context::new();

    if !get_setting::<bool, _>("allow_registrations", &mut transaction).await? {
        return Err(HttpError(403, "User registrations are disabled".to_owned()).into());
    }

    if let Some(site_key) = get_optional_setting::<String, _>("hcaptcha.site_key", &mut transaction).await? {
        context.try_insert("hcaptcha_site_key", &site_key)?;
    }

    render_template!("user/register.html", context, transaction)
}

#[route("/api/user", method = "POST")]
pub(crate) async fn post_register(body: web::Json<RegisterJsonRequest>, id: Identity, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    if id.identity().is_some() {
        // Maybe just redirect to home page?
        return Err(HttpError(401, "Already logged in".to_owned()).into());
    }

    let mut transaction = db_pool.begin().await?;

    if !get_setting::<bool, _>("allow_registrations", &mut transaction).await? {
        return Err(HttpError(403, "User registrations are disabled".to_owned()).into());
    }

    let username = &body.username;

    if username.len() < 3 || username.len() > 32 || !username.chars().all(|c| is_valid(&c)) {
        return Err(HttpError(400, "Username must be between 3 and 32 characters long and may only contain a-z, 0-9, _ or -".to_owned()).into());
    }

    if is_reserved_username(username.as_str()) {
        return Err(HttpError(400, "Username is a reserved identifier".to_owned()).into());
    }

    if !is_fs_legal(username) {
        return Err(HttpError(400, "Username is illegal".to_owned()).into());
    }

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from users where lower(username) = lower($1));")
        .bind(username)
        .fetch_one(&mut transaction)
        .await?;

    if exists {
        return Err(HttpError(409, "Username already in use".to_owned()).into());
    }

    let email = &body.email;

    // This is not according to the spec of the IETF but trying to implement that is honestly out-of-bounds for this project
    // Thus a best effort naive implementation. Checks for the presence of "@" and a "." in the domain name (after the last @)
    if !email.contains('@') || !email.rsplit_once("@").map(|(_, x)| x).unwrap_or_default().contains('.') {
        return Err(HttpError(400, "Invalid email address".to_owned()).into());
    }

    let raw_password = &body.password;

    // We don't implement any strict password rules according to NIST 2017 Guidelines
    if raw_password.len() < 8 {
        return Err(HttpError(400, "Password must be at least 8 characters".to_owned()).into());
    }

    let password = crypto::hash_password(raw_password)?;

    let captcha_success = captcha::verify_captcha(&body.h_captcha_response.to_owned(), &mut transaction).await?;

    if !captcha_success {
        return Err(HttpError(422, "Captcha verification failed".to_owned()).into());
    }

    let user: User = sqlx::query_as::<_, User>("insert into users (username, email, password) values ($1, $2, $3) returning *")
        .bind(username)
        .bind(email)
        .bind(&password)
        .fetch_one(&mut transaction)
        .await?;

    send_verification_mail(&user, &db_pool).await?;

    let session = Session::new(&request, &user, &mut transaction).await?;
    id.remember(session.to_string());

    transaction.commit().await?;

    info!("New user registered: {} (id {})", &user.username, &user.id);

    Ok(if request.get_header("hx-request").is_some() {
        HttpResponse::Ok().header("hx-redirect", "/").header("hx-refresh", "true").finish()
    } else {
        HttpResponse::Ok().json(RegisterJsonResponse {
            success: true,
            id: user.id
        })
    })
}

#[derive(Deserialize)]
pub(crate) struct RegisterJsonRequest {
    username: String,
    email: String,
    password: String,
    #[serde(rename = "h-captcha-response")]
    h_captcha_response: String
}

#[derive(Serialize)]
struct RegisterJsonResponse {
    success: bool,
    id: i32
}
