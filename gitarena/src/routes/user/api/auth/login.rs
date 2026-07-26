use crate::database::Pool;
use crate::mail::Email;
use crate::routes::user::api::auth::me::MeResponse;
use crate::session::{Session, send_login_email};
use crate::user::{User, WebUser};
use crate::{crypto, die, err};

use crate::events::Event;
use actix_identity::Identity;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use fang::AsyncQueue;
use gitarena_macros::route;
use serde::Deserialize;
use serde_json::json;
use tracing::debug;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginJsonRequest,
    responses(
        (status = 200, description = "Login successful", body = MeResponse),
        (status = 400, description = "Empty identifier or password"),
        (status = 401, description = "Invalid credentials or SSO account"),
        (status = 403, description = "Account disabled"),
        (status = 409, description = "Already logged in"),
    ),
    tag = "user"
)]
#[route("/api/auth/login", method = "POST", err = "json")]
pub(crate) async fn post_login(
    body: web::Json<LoginJsonRequest>,
    web_user: WebUser,
    request: HttpRequest,
    queue: web::Data<AsyncQueue>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    if matches!(web_user, WebUser::Authenticated(_)) {
        die!(CONFLICT, "Already logged in");
    }

    let identifier = body.identifier.trim();
    let password = &body.password;

    if identifier.is_empty() {
        die!(BAD_REQUEST, "Username or email cannot be empty");
    }

    if password.is_empty() {
        die!(BAD_REQUEST, "Password cannot be empty");
    }

    let mut transaction = db_pool.begin().await?;
    let first_cred;

    let option: Option<User> = if identifier.contains('@') {
        first_cred = "E-Mail";
        User::find_using_email(identifier, &mut transaction).await
    } else {
        first_cred = "Username";
        User::find_using_name(identifier, &mut transaction).await
    };

    let Some(user) = option else {
        debug!(identifier, "Received login request for non-existent user");
        die!(UNAUTHORIZED, "Invalid credentials");
    };

    if user.password == "sso-login" {
        debug!(user.username, user.id = %user.id, "Received password login request for an SSO-registered user");
        die!(UNAUTHORIZED, "This account was registered with SSO. Use an SSO provider to sign in.");
    }

    if !crypto::check_password(&user, password)? {
        Event::new("auth.login_failed", Uuid::nil(), &request, (&user).into(), None)
            .save(&mut transaction)
            .await?;

        debug!(user.username, user.id = %user.id, "Received login request with wrong password");
        die!(UNAUTHORIZED, "Invalid credentials");
    }

    let primary_email = Email::find_primary_email(user.id, &mut transaction)
        .await?
        .ok_or_else(|| err!(UNAUTHORIZED, "No primary email"))?;

    if user.disabled {
        debug!(user.username, user.id = %user.id, "Received login request for disabled user");
        die!(FORBIDDEN, "Account has been disabled. Please contact support.");
    }

    if !primary_email.is_allowed_login() {
        debug!(user.username, user.id = %user.id, "Received login request from unverified user");
        die!(FORBIDDEN, "Verify your email address before logging in.");
    }

    let session = Session::new(&request, &user, &mut transaction).await?;
    Identity::login(&*request.extensions(), session.to_string())?;

    Event::new(
        "auth.login",
        user.id,
        &request,
        (&user).into(),
        Some(json!({
            "type": "password"
        })),
    )
    .save(&mut transaction)
    .await?;

    debug!(user.username, user.id = %user.id, "User logged in");

    transaction.commit().await?;

    send_login_email(&user, &format!("{first_cred} and password"), &request, &queue, &db_pool).await?;

    Ok(HttpResponse::Ok().json(MeResponse {
        id: user.id,
        username: user.username,
        admin: user.admin,
    }))
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct LoginJsonRequest {
    /// Username or email address
    identifier: String,
    password: String,
}
