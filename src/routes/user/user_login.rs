use crate::crypto;
use crate::error::GAErrors::HttpError;
use crate::user::User;
use crate::verification;

use actix_identity::Identity;
use actix_web::http::header;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use lazy_static::lazy_static;
use serde::Deserialize;
use sqlx::PgPool;
use tracing_unwrap::OptionExt;

lazy_static! {
    static ref ROOT_PATH: String = "/".to_owned();
}

#[route("/login", method="POST")]
pub(crate) async fn login(body: web::Form<LoginRequest>, id: Identity, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    if id.identity().is_some() {
        // Maybe just redirect to home page?
        return Err(HttpError(401, "Already logged in".to_owned()).into());
    }

    let username = &body.username;
    let password = &body.password;

    if username.is_empty() || password.is_empty() {
        return Err(HttpError(400, "Username or password cannot be empty".to_owned()).into());
    }

    let mut transaction = db_pool.begin().await?;

    let option: Option<User> = sqlx::query_as::<_, User>("select * from users where username = $1 limit 1")
        .bind(username)
        .fetch_optional(&mut transaction)
        .await?;

    if option.is_none() {
        return Err(HttpError(401, "Incorrect username or password".to_owned()).into());
    }

    let user = option.unwrap_or_log();

    if !crypto::check_password(&user, &password)? {
        return Err(HttpError(401, "Incorrect username or password".to_owned()).into());
    }

    if user.disabled || verification::is_pending(&user, &mut transaction).await? {
        return Err(HttpError(401, "Account has been disabled".to_owned()).into());
    }

    id.remember(user.identity_str());

    transaction.commit().await?;

    let redirect = match &body.redirect {
        Some(path) => path,
        None => &ROOT_PATH,
    };

    Ok(HttpResponse::Found()
        .header(header::LOCATION, redirect.as_str())
        .finish())
}

#[derive(Deserialize)]
pub(crate) struct LoginRequest {
    username: String,
    password: String,
    redirect: Option<String>
}
