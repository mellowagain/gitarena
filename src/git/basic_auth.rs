use crate::{crypto, die, err};
use crate::git::basic_auth;
use crate::prelude::*;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::user::User;

use actix_web::http::header::{CONTENT_TYPE, WWW_AUTHENTICATE};
use actix_web::{Either, HttpRequest, HttpResponse};
use anyhow::Result;
use sqlx::{Executor, Postgres};
use tracing::instrument;
use tracing_unwrap::OptionExt;

#[instrument(skip(request, executor), err)]
pub(crate) async fn validate_repo_access<'e, E>(repo: Option<Repository>, content_type: &str, request: &HttpRequest, executor: E) -> Result<Either<(Option<User>, Repository), HttpResponse>>
    where E: Executor<'e, Database = Postgres>
{
    match repo {
        Some(repo) => {
            if repo.visibility != RepoVisibility::Public {
                return match login_flow(request, executor, content_type).await? {
                    Either::Left(user) => Ok(Either::Left((Some(user), repo))),
                    Either::Right(response) => Ok(Either::Right(response))
                }
            }

            Ok(Either::Left((None, repo)))
        },
        None => {
            // Prompt for authentication even if the repo does not exist to prevent leakage of private repositories
            let _ = login_flow(request, executor, content_type).await?;

            die!(NOT_FOUND, "Repository not found");
        }
    }
}

#[instrument(skip(request, executor), err)]
pub(crate) async fn login_flow<'e, E>(request: &HttpRequest, executor: E, content_type: &str) -> Result<Either<User, HttpResponse>>
    where E: Executor<'e, Database = Postgres>
{
    if !is_present(request).await {
        return Ok(Either::Right(prompt(content_type).await));
    }

    Ok(Either::Left(authenticate(request, executor).await?))
}

#[instrument]
pub(crate) async fn prompt(content_type: &str) -> HttpResponse {
    HttpResponse::Unauthorized()
        .append_header((CONTENT_TYPE, content_type))
        .append_header((WWW_AUTHENTICATE, "Basic realm=\"GitArena\", charset=\"UTF-8\""))
        .finish()
}

#[instrument(skip_all, err)]
pub(crate) async fn authenticate<'e, E>(request: &HttpRequest, transaction: E) -> Result<User>
    where E: Executor<'e, Database = Postgres>
{
    // TODO: Add more verbose logging to this function similar to frontend login (for usage by fail2ban)

    match request.get_header("authorization") {
        Some(auth_header) => {
            let (username, password) = parse_basic_auth(auth_header).await?;

            if username.is_empty() || password.is_empty() {
                die!(UNAUTHORIZED, "Username and password cannot be empty");
            }

            let option: Option<User> = sqlx::query_as::<_, User>("select * from users where username = $1 limit 1")
                .bind(&username)
                .fetch_optional(transaction)
                .await?;

            if option.is_none() {
                die!(UNAUTHORIZED, "User does not exist");
            }

            let user = option.unwrap_or_log();

            if !crypto::check_password(&user, &password)? {
                die!(UNAUTHORIZED, "Incorrect password");
            }

            // TODO: Check for allowed login
            /*let primary_email = Email::find_primary_email(&user, transaction)
                .await?
                .ok_or_else(|| anyhow!("No primary email".to_owned()))?;*/

            if user.disabled/* || !primary_email.is_allowed_login()*/ {
                die!(UNAUTHORIZED, "Account has been disabled. Please contact support.");
            }

            Ok(user)
        }
        None => die!(UNAUTHORIZED)
    }
}

#[instrument(skip(auth_header), err)]
pub(crate) async fn parse_basic_auth(auth_header: &str) -> Result<(String, String)> {
    let (auth_type, base64_credentials) = auth_header.split_once(' ').ok_or(|| err!(UNAUTHORIZED))?;

    if auth_type != "Basic" {
        die!(UNAUTHORIZED, "Unsupported authentication type, only Basic auth allowed");
    }

    let credentials = String::from_utf8(base64::decode(base64_credentials)?)?;

    Ok(credentials.split_once(':')
        .map(|(username, password)| (username.to_owned(), password.to_owned()))
        .ok_or(|| err!(UNAUTHORIZED, "Both username and password is required"))?)
}

pub(crate) async fn is_present(request: &HttpRequest) -> bool {
    request.get_header("authorization").is_some()
}
