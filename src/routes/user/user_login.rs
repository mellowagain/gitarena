use crate::mail::Email;
use crate::render_template;
use crate::session::Session;
use crate::user::{User, WebUser};
use crate::{crypto, die, err};

use actix_identity::Identity;
use actix_web::http::header::LOCATION;
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::{from_config, route};
use serde::Deserialize;
use sqlx::PgPool;
use tera::Context;
use tracing_unwrap::OptionExt;
use log::debug;

#[route("/login", method = "GET", err = "html")]
pub(crate) async fn get_login(web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    if matches!(web_user, WebUser::Authenticated(_)) {
        die!(UNAUTHORIZED, "Already logged in");
    }

    let (allow_registrations, bitbucket_sso_enabled, github_sso_enabled, gitlab_sso_enabled): (bool, bool, bool, bool) = from_config!(
        "allow_registrations" => bool,
        "sso.bitbucket.enabled" => bool,
        "sso.github.enabled" => bool,
        "sso.gitlab.enabled" => bool
    );

    let mut context = Context::new();

    context.try_insert("allow_registrations", &allow_registrations)?;
    context.try_insert("sso_bitbucket", &bitbucket_sso_enabled)?;
    context.try_insert("sso_github", &github_sso_enabled)?;
    context.try_insert("sso_gitlab", &gitlab_sso_enabled)?;

    render_template!("user/login.html", context)
}

#[route("/login", method = "POST", err = "html")]
pub(crate) async fn post_login(body: web::Form<LoginRequest>, request: HttpRequest, id: Identity, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let redirect = body.redirect.as_deref().unwrap_or("/");

    // User is already logged in
    if id.identity().is_some() {
        return Ok(HttpResponse::Found().header(LOCATION, redirect).finish());
    }

    // TODO: Maybe allow login with email address?
    let username = &body.username;
    let password = &body.password;

    let mut context = Context::new();
    context.try_insert("username", username.as_str())?;
    context.try_insert("password", password.as_str())?;
    context.try_insert("error", &true)?; // The login template only gets rendered if an error occurs

    if username.is_empty() {
        context.try_insert("username_error", "Username cannot be empty")?;
        return render_template!(StatusCode::BAD_REQUEST, "user/login.html", context);
    }

    if password.is_empty() {
        context.try_insert("password_error", "Password cannot be empty")?;
        return render_template!(StatusCode::BAD_REQUEST, "user/login.html", context);
    }

    // We specify whenever a username does not exist or if the password was incorrect
    // This is by design as one can check anytime by just going to /<username> or checking the sign-up form
    // Please see https://meta.stackoverflow.com/q/308782

    let mut transaction = db_pool.begin().await?;

    let option: Option<User> = sqlx::query_as::<_, User>("select * from users where username = $1 limit 1")
        .bind(username)
        .fetch_optional(&mut transaction)
        .await?;

    if option.is_none() {
        debug!("Received login request for non-existent user: {}", &username);

        context.try_insert("username_error", "Username does not exist")?;
        return render_template!(StatusCode::UNAUTHORIZED, "user/login.html", context, transaction);
    }

    let user = option.unwrap_or_log();

    if user.password == "sso-login" {
        debug!("Received login request for an {} (id {}) despite being registered with SSO", &user.username, &user.id);

        context.try_insert("password_error", "Your account has been registered with SSO. Try using another login method below.")?;
        return render_template!(StatusCode::UNAUTHORIZED, "user/login.html", context, transaction);
    }

    if !crypto::check_password(&user, password)? {
        debug!("Received login request with wrong password for {} (id {})", &user.username, &user.id);

        context.try_insert("password_error", "Incorrect password")?;
        return render_template!(StatusCode::UNAUTHORIZED, "user/login.html", context, transaction);
    }

    let primary_email = Email::find_primary_email(&user, &mut transaction)
        .await?
        .ok_or_else(|| err!(UNAUTHORIZED, "No primary email"))?;

    if user.disabled || !primary_email.is_allowed_login() {
        debug!("Received login request for disabled user {} (id {})", &user.username, &user.id);

        context.try_insert("general_error", "Account has been disabled. Please contact support.")?;
        return render_template!(StatusCode::UNAUTHORIZED, "user/login.html", context, transaction);
    }

    let session = Session::new(&request, &user, &mut transaction).await?;
    id.remember(session.to_string());

    debug!("{} (id {}) logged in successfully", &user.username, &user.id);

    transaction.commit().await?;

    Ok(HttpResponse::Found().header(LOCATION, redirect).finish())
}

#[derive(Deserialize)]
pub(crate) struct LoginRequest {
    username: String,
    password: String,
    redirect: Option<String>
}
