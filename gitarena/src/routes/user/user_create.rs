use crate::config::{get_optional_setting, get_setting};
use crate::session::Session;
use crate::user::User;
use crate::utils::identifiers::{is_username_taken, validate_username};
use crate::verification::send_verification_mail;
use crate::{captcha, crypto, die};
use gitarena_common::database::Pool;

use crate::mail::Email;
use actix_identity::Identity;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use fang::AsyncQueue;
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

#[route("/api/user", method = "POST", err = "json")]
pub(crate) async fn post_register(
    body: web::Json<RegisterJsonRequest>,
    id: Identity,
    request: HttpRequest,
    queue: web::Data<AsyncQueue>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    if id.identity().is_some() {
        // Maybe just redirect to home page?
        die!(UNAUTHORIZED, "Already logged in");
    }

    let mut tx = db_pool.begin().await?;

    let allow_registrations: bool = get_setting("allow_registrations", &mut tx).await?;

    if !allow_registrations {
        die!(FORBIDDEN, "User registrations are disabled");
    }

    let username = &body.username;

    validate_username(username.as_str())?;

    if is_username_taken(username.as_str(), &mut tx).await? {
        die!(CONFLICT, "Username already in use");
    }

    let email = &body.email;

    // This is not according to the spec of the IETF but trying to implement that is honestly out-of-bounds for this project
    // Thus a best effort naive implementation. Checks for the presence of "@" and a "." in the domain name (after the last @)
    if !email.contains('@') || !email.rsplit_once('@').map(|(_, x)| x).unwrap_or_default().contains('.') {
        die!(BAD_REQUEST, "Invalid email address");
    }

    let (email_exists,): (bool,) = sqlx::query_as("select exists(select 1 from emails where lower(email) = lower($1) limit 1)")
        .bind(email)
        .fetch_one(&mut *tx)
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

    if get_optional_setting::<String>("hcaptcha.site_key", &mut tx).await?.is_some() {
        if let Some(h_captcha_response) = &body.h_captcha_response {
            let captcha_success = captcha::verify_captcha(h_captcha_response, &mut tx).await?;

            if !captcha_success {
                die!(UNPROCESSABLE_ENTITY, "Captcha verification failed");
            }
        } else {
            die!(BAD_REQUEST, "HCaptcha response was not provided");
        }
    }

    let user: User = sqlx::query_as::<_, User>("insert into users (id, username, password) values ($1, $2, $3) returning *")
        .bind(Uuid::now_v7())
        .bind(username)
        .bind(&password)
        .fetch_one(&mut *tx)
        .await?;

    let email = sqlx::query_as::<_, Email>(
        "insert into emails (id, owner, email, \"primary\", commit, notification, public) values ($1, $2, $3, true, true, true, true) returning *",
    )
    .bind(Uuid::now_v7())
    .bind(user.id)
    .bind(email)
    .fetch_one(&mut *tx)
    .await?;

    send_verification_mail(&user, email.email, &queue, &db_pool).await?;

    // Close the transaction so the email gets committed (above) and then immediately start a new one for `session` below
    tx.commit().await?;

    let mut tx = db_pool.begin().await?;

    let session = Session::new(&request, &user, &mut tx).await?;
    id.remember(session.to_string());

    tx.commit().await?;

    info!(user.username, user.id = %user.id, "New user signed up");

    Ok(HttpResponse::Ok().json(RegisterJsonResponse { success: true, id: user.id }))
}

#[derive(Deserialize)]
pub(crate) struct RegisterJsonRequest {
    username: String,
    email: String,
    password: String,
    #[serde(rename = "h-captcha-response")]
    h_captcha_response: Option<String>,
}

#[derive(Serialize)]
struct RegisterJsonResponse {
    success: bool,
    id: Uuid,
}
