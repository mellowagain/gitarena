use crate::mail::Email;
use crate::user::WebUser;
use crate::verification::send_verification_mail;
use crate::{die, err};
use gitarena_common::database::Pool;
use gitarena_macros::{from_config, route};

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use fang::AsyncQueue;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/emails",
    responses(
        (status = 200, description = "List of emails for the authenticated user", body = Vec<Email>),
        (status = 401, description = "Authentication required"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/emails", method = "GET", err = "json")]
pub(crate) async fn get_emails(web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let mut transaction = db_pool.begin().await?;

    let emails = sqlx::query_as::<_, Email>("select * from emails where owner = $1 order by id")
        .bind(user.id)
        .fetch_all(&mut *transaction)
        .await?;

    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(emails))
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct AddEmailRequest {
    email: String,
}

#[utoipa::path(
    post,
    path = "/api/emails",
    request_body = AddEmailRequest,
    responses(
        (status = 201, description = "Email added", body = Email),
        (status = 400, description = "Invalid email"),
        (status = 401, description = "Authentication required"),
        (status = 409, description = "Email already exists"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/emails", method = "POST", err = "json")]
pub(crate) async fn post_email(
    body: web::Json<AddEmailRequest>,
    web_user: WebUser,
    queue: web::Data<AsyncQueue>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if body.email.is_empty() {
        die!(BAD_REQUEST, "Email address is required");
    }

    if !body.email.contains('@') {
        die!(BAD_REQUEST, "Invalid email address");
    }

    let smtp_enabled = from_config!("smtp.enabled" => bool);
    let verified_filter = if smtp_enabled { " and verified_at is not null" } else { "" };

    let mut transaction = db_pool.begin().await?;

    let (exists,): (bool,) = sqlx::query_as(&format!("select exists(select 1 from emails where email = $1{verified_filter} limit 1)"))
        .bind(body.email.as_str())
        .fetch_one(&mut *transaction)
        .await?;

    if exists {
        die!(CONFLICT, "Email address already in use");
    }

    let email = sqlx::query_as::<_, Email>("insert into emails (id, owner, email) values ($1, $2, $3) returning *")
        .bind(Uuid::now_v7())
        .bind(user.id)
        .bind(body.email.as_str())
        .fetch_one(&mut *transaction)
        .await?;

    transaction.commit().await?;

    send_verification_mail(&user, email.email.clone(), &queue, &db_pool).await?;

    Ok(HttpResponse::Created().json(email))
}

#[utoipa::path(
    delete,
    path = "/api/emails/{id}",
    params(("id" = Uuid, Path, description = "Email ID")),
    responses(
        (status = 204, description = "Email deleted"),
        (status = 400, description = "Cannot remove primary email"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Email not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/emails/{id}", method = "DELETE", err = "json")]
pub(crate) async fn delete_email(path: web::Path<Uuid>, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let email_id = path.into_inner();

    let mut transaction = db_pool.begin().await?;

    let email = sqlx::query_as::<_, Email>("select * from emails where id = $1 and owner = $2 limit 1")
        .bind(email_id)
        .bind(user.id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| err!(NOT_FOUND, "Email not found"))?;

    if email.primary {
        die!(BAD_REQUEST, "Cannot remove the primary email address");
    }

    sqlx::query("delete from emails where id = $1")
        .bind(email.id)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct UpdateEmailRequest {
    /// Set as the primary email (requires the address to be verified)
    primary: Option<bool>,
    /// Receive notifications at this address
    notification: Option<bool>,
    /// Show this address publicly
    public: Option<bool>,
}

#[utoipa::path(
    patch,
    path = "/api/emails/{id}",
    params(("id" = Uuid, Path, description = "Email ID")),
    request_body = UpdateEmailRequest,
    responses(
        (status = 200, description = "Email updated", body = Email),
        (status = 400, description = "Unverified email cannot be set as primary"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Email not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/emails/{id}", method = "PATCH", err = "json")]
pub(crate) async fn patch_email(
    path: web::Path<Uuid>,
    body: web::Json<UpdateEmailRequest>,
    web_user: WebUser,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let email_id = path.into_inner();

    let mut transaction = db_pool.begin().await?;

    let email = sqlx::query_as::<_, Email>("select * from emails where id = $1 and owner = $2 limit 1")
        .bind(email_id)
        .bind(user.id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| err!(NOT_FOUND, "Email not found"))?;

    if let Some(true) = body.primary {
        if email.verified_at.is_none() {
            die!(BAD_REQUEST, "Email must be verified before setting it as primary");
        }

        sqlx::query("update emails set \"primary\" = false where owner = $1")
            .bind(user.id)
            .execute(&mut *transaction)
            .await?;
    }

    if let Some(true) = body.notification {
        sqlx::query("update emails set notification = false where owner = $1")
            .bind(user.id)
            .execute(&mut *transaction)
            .await?;
    }

    if let Some(true) = body.public {
        sqlx::query("update emails set public = false where owner = $1")
            .bind(user.id)
            .execute(&mut *transaction)
            .await?;
    }

    let new_primary = body.primary.unwrap_or(email.primary);
    let new_notification = body.notification.unwrap_or(email.notification);
    let new_public = body.public.unwrap_or(email.public);

    let updated = sqlx::query_as::<_, Email>("update emails set \"primary\" = $1, notification = $2, public = $3 where id = $4 returning *")
        .bind(new_primary)
        .bind(new_notification)
        .bind(new_public)
        .bind(email_id)
        .fetch_one(&mut *transaction)
        .await?;

    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(updated))
}

#[utoipa::path(
    post,
    path = "/api/emails/{id}/verify",
    params(("id" = Uuid, Path, description = "Email ID")),
    responses(
        (status = 204, description = "Verification email sent (or SMTP disabled)"),
        (status = 400, description = "Email is already verified"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Email not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/emails/{id}/verify", method = "POST", err = "json")]
pub(crate) async fn resend_verify_email(
    path: web::Path<Uuid>,
    web_user: WebUser,
    queue: web::Data<AsyncQueue>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let email_id = path.into_inner();

    let mut transaction = db_pool.begin().await?;

    let email = sqlx::query_as::<_, Email>("select * from emails where id = $1 and owner = $2 limit 1")
        .bind(email_id)
        .bind(user.id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| err!(NOT_FOUND, "Email not found"))?;

    if email.verified_at.is_some() {
        die!(BAD_REQUEST, "Email is already verified");
    }

    transaction.commit().await?;

    send_verification_mail(&user, email.email.clone(), &queue, &db_pool).await?;

    Ok(HttpResponse::NoContent().finish())
}
