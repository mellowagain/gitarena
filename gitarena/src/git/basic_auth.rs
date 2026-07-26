use crate::prelude::*;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::user::User;
use crate::{crypto, die, err};

use crate::database::Database;
use crate::mail::Email;
use actix_web::http::header::{CONTENT_TYPE, WWW_AUTHENTICATE};
use actix_web::{HttpRequest, HttpResponse};
use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose;
use either::Either;
use sqlx::Transaction;
use tracing::{debug, instrument};

#[instrument(skip(request, tx), err)]
pub(crate) async fn validate_repo_access(
    repo: Option<Repository>,
    content_type: &str,
    request: &HttpRequest,
    tx: &mut Transaction<'_, Database>,
) -> Result<Either<(Option<User>, Repository), HttpResponse>> {
    let Some(repo) = repo else {
        // Prompt for authentication even if the repo does not exist to prevent leakage of private repositories
        let _ = login_flow(request, tx, content_type).await?;

        die!(NOT_FOUND, "Repository not found");
    };

    if repo.visibility != RepoVisibility::Public {
        return match login_flow(request, tx, content_type).await? {
            Either::Left(user) => Ok(Either::Left((Some(user), repo))),
            Either::Right(response) => Ok(Either::Right(response)),
        };
    }

    Ok(Either::Left((None, repo)))
}

#[instrument(skip(request, tx), err)]
pub(crate) async fn login_flow(request: &HttpRequest, tx: &mut Transaction<'_, Database>, content_type: &str) -> Result<Either<User, HttpResponse>> {
    if !is_present(request) {
        return Ok(Either::Right(prompt(content_type).await));
    }

    Ok(Either::Left(authenticate(request, tx).await?))
}

#[instrument]
pub(crate) async fn prompt(content_type: &str) -> HttpResponse {
    HttpResponse::Unauthorized()
        .append_header((CONTENT_TYPE, content_type))
        .append_header((WWW_AUTHENTICATE, "Basic realm=\"GitArena\", charset=\"UTF-8\""))
        .finish()
}

#[instrument(skip_all, err)]
pub(crate) async fn authenticate(request: &HttpRequest, transaction: &mut Transaction<'_, Database>) -> Result<User> {
    match request.get_header("authorization") {
        Some(auth_header) => {
            let (identifier, password) = parse_basic_auth(auth_header).await?;

            if identifier.is_empty() || password.is_empty() {
                die!(UNAUTHORIZED, "Username and password cannot be empty");
            }

            let option = if identifier.contains('@') {
                User::find_using_email(&identifier, transaction).await
            } else {
                User::find_using_name(&identifier, transaction).await
            };

            let Some(user) = option else {
                debug!(identifier, "Received git http login request for non-existent user");
                die!(UNAUTHORIZED, "Invalid credentials");
            };

            if user.password == "sso-login" {
                debug!(user.username, user.id = %user.id, "Received git http password login request for an SSO-registered user");
                die!(UNAUTHORIZED, "This account was registered with SSO. Only Git operations via SSH are supported.");
            }

            if !crypto::check_password(&user, &password)? {
                die!(UNAUTHORIZED, "Invalid credentials");
            }

            let primary_email = Email::find_primary_email(user.id, transaction)
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

            debug!(user.username, user.id = %user.id, "User authenticated in git http");
            Ok(user)
        }
        None => die!(UNAUTHORIZED),
    }
}

#[instrument(skip(auth_header), err)]
pub(crate) async fn parse_basic_auth(auth_header: &str) -> Result<(String, String)> {
    let (auth_type, base64_credentials) = auth_header.split_once(' ').ok_or_else(|| err!(BAD_REQUEST))?;

    if auth_type != "Basic" {
        die!(UNAUTHORIZED, "Unsupported authentication type, only Basic auth allowed");
    }

    let credentials = String::from_utf8(general_purpose::STANDARD.decode(base64_credentials)?)?;

    Ok(credentials
        .split_once(':')
        .map(|(username, password)| (username.to_owned(), password.to_owned()))
        .ok_or_else(|| err!(UNAUTHORIZED, "Both username and password is required"))?)
}

pub(crate) fn is_present(request: &HttpRequest) -> bool {
    request.get_header("authorization").is_some()
}
