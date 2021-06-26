use crate::user::User;
use crate::{PgPoolConnection, templates, GaE};

use actix_web::{HttpResponse, post, Responder, Result as ActixResult, web};
use anyhow::{Result};
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgQueryAs;
use sqlx::{PgPool, Transaction};

// POST /api/users
async fn register(body: web::Json<RegisterJsonRequest>, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction: Transaction<PgPoolConnection> = db_pool.begin().await?;

    let username = &body.username;

    if username.len() < 3 || !username.chars().all(|c| is_username(&c)) {
        return Ok(HttpResponse::BadRequest().json(RegisterJsonResponse {
            success: false,
            id: None,
            errors: Some("Username must be at least 3 characters and may only contain a-z, 0-9, _ or -".to_owned())
        }).await);
    }

    let lowered_username = username.to_lowercase();

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from users where lower(username) = $1);")
        .bind(&lowered_username)
        .fetch_one(&mut transaction)
        .await?;

    if exists {
        return Ok(HttpResponse::Conflict().json(RegisterJsonResponse {
            success: false,
            id: None,
            errors: Some("Username already in use".to_owned())
        }).await);
    }

    let captcha_success = true/*bail!(captcha::verify_captcha(&body.h_captcha_response.to_owned()).await)*/;

    if !captcha_success {
        return Ok(HttpResponse::UnprocessableEntity().json(RegisterJsonResponse {
            success: false,
            id: None,
            errors: Some("Captcha verification failed".to_owned())
        }).await);
    }

    let mut user: User = User::new(
        username.to_owned(), body.email.to_owned(), body.password.to_owned()
    )?;
    user.save(db_pool.get_ref()).await?;

    transaction.commit().await?;

    user.send_template(&templates::VERIFY_EMAIL, Some([
            ("username".to_owned(), user.username.to_owned()),
            ("link".to_owned(), "bruuh4".to_owned())
    ].iter().cloned().collect())).await?;

    info!("New user registered: {} (id {})", user.username, user.id);

    Ok(HttpResponse::Ok().json(RegisterJsonResponse {
        success: true,
        id: Some(user.id),
        errors: None
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
    id: Option<i32>,
    errors: Option<String>
}

#[post("/api/user")]
pub(crate) async fn handle_post(body: web::Json<RegisterJsonRequest>, db_pool: web::Data<PgPool>) -> ActixResult<impl Responder> {
    Ok(register(body, db_pool).await.map_err(|e| -> GaE { e.into() }))
}
