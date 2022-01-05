use crate::config::{get_optional_setting, get_setting};
use crate::prelude::*;
use crate::session::Session;
use crate::user::{User, WebUser};
use crate::utils::identifiers::{is_username_taken, validate_username};
use crate::verification::send_verification_mail;
use crate::{captcha, crypto, die, render_template};

use actix_identity::Identity;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tera::Context;

#[route("/register", method = "GET", err = "html")]
pub(crate) async fn get_register(web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    if matches!(web_user, WebUser::Authenticated(_)) {
        die!(UNAUTHORIZED, "Already logged in");
    }

    let mut context = Context::new();

    if !get_setting::<bool, _>("allow_registrations", &mut transaction).await? {
        die!(FORBIDDEN, "User registrations are disabled");
    }

    if let Some(site_key) = get_optional_setting::<String, _>("hcaptcha.site_key", &mut transaction).await? {
        context.try_insert("hcaptcha_site_key", &site_key)?;
    }

    render_template!("user/register.html", context, transaction)
}

#[route("/api/user", method = "POST", err = "htmx+html")]
pub(crate) async fn post_register(body: web::Json<RegisterJsonRequest>, id: Identity, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    if id.identity().is_some() {
        // Maybe just redirect to home page?
        die!(UNAUTHORIZED, "Already logged in");
    }

    let mut transaction = db_pool.begin().await?;

    if !get_setting::<bool, _>("allow_registrations", &mut transaction).await? {
        die!(FORBIDDEN, "User registrations are disabled");
    }

    let username = &body.username;

    validate_username(username.as_str())?;

    if is_username_taken(username.as_str(), &mut transaction).await? {
        die!(CONFLICT, "Username already in use");
    }

    let email = &body.email;

    // This is not according to the spec of the IETF but trying to implement that is honestly out-of-bounds for this project
    // Thus a best effort naive implementation. Checks for the presence of "@" and a "." in the domain name (after the last @)
    if !email.contains('@') || !email.rsplit_once("@").map(|(_, x)| x).unwrap_or_default().contains('.') {
        die!(BAD_REQUEST, "Invalid email address");
    }

    let (email_exists,): (bool,) = sqlx::query_as("select exists(select 1 from emails where lower(email) = lower($1) limit 1)")
        .bind(email)
        .fetch_one(&mut transaction)
        .await?;

    if email_exists {
        die!(CONFLICT, "Email already in use");
    }

    let raw_password = &body.password;

    // We don't implement any strict password rules according to NIST 2017 Guidelines
    // TODO: Allow configuration of password rules
    if raw_password.len() < 8 {
        die!(BAD_REQUEST, "Password must be at least 8 characters");
    }

    let password = crypto::hash_password(raw_password)?;

    if get_optional_setting::<String, _>("hcaptcha.site_key", &mut transaction).await?.is_some() {
        if let Some(h_captcha_response) = &body.h_captcha_response {
            let captcha_success = captcha::verify_captcha(h_captcha_response, &mut transaction).await?;

            if !captcha_success {
                die!(UNPROCESSABLE_ENTITY, "Captcha verification failed");
            }
        } else {
            die!(BAD_REQUEST, "HCaptcha response was not provided");
        }
    }

    let user: User = sqlx::query_as::<_, User>("insert into users (username, password) values ($1, $2) returning *")
        .bind(username)
        .bind(&password)
        .fetch_one(&mut transaction)
        .await?;

    sqlx::query("insert into emails (owner, email, \"primary\", commit, notification, public) values ($1, $2, true, true, true, true)")
        .bind(&user.id)
        .bind(email)
        .execute(&mut transaction)
        .await?;

    // Close the transaction so the email gets committed (above) and then immediatly start a new one for `session` below
    transaction.commit().await?;
    let mut transaction = db_pool.begin().await?;

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
    h_captcha_response: Option<String>
}

#[derive(Serialize)]
struct RegisterJsonResponse {
    success: bool,
    id: i32
}
