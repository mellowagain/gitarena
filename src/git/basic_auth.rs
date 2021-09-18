use crate::crypto;
use crate::error::GAErrors::GitError;
use crate::extensions::get_header;
use crate::git::basic_auth;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::user::User;

use actix_web::{Either, HttpRequest, HttpResponse};
use anyhow::Result;
use sqlx::{Postgres, Transaction};

pub(crate) async fn validate_repo_access(repo: Option<Repository>, content_type: &str, request: &HttpRequest, transaction: &mut Transaction<'_, Postgres>) -> Result<Either<(Option<User>, Repository), HttpResponse>> {
    match repo {
        Some(repo) => {
            if repo.visibility != RepoVisibility::Public {
                return match login_flow(request, transaction, content_type).await? {
                    Either::A(user) => Ok(Either::A((Some(user), repo))),
                    Either::B(response) => Ok(Either::B(response))
                }
            }

            Ok(Either::A((None, repo)))
        },
        None => {
            // Prompt for authentication even if the repo does not exist to prevent leakage of private repositories
            let _ = login_flow(request, transaction, content_type).await?;

            Err(GitError(404, None).into())
        }
    }
}

pub(crate) async fn login_flow(request: &HttpRequest, transaction: &mut Transaction<'_, Postgres>, content_type: &str) -> Result<Either<User, HttpResponse>> {
    if !basic_auth::is_present(&request).await {
        return Ok(Either::B(prompt(content_type).await));
    }

    Ok(Either::A(basic_auth::authenticate(&request, transaction).await?))
}

pub(crate) async fn prompt(content_type: &str) -> HttpResponse {
    HttpResponse::Unauthorized()
        .header("Content-Type", content_type)
        .header("WWW-Authenticate", "Basic realm=\"GitArena\", charset=\"UTF-8\"")
        .finish()
}

pub(crate) async fn authenticate(request: &HttpRequest, transaction: &mut Transaction<'_, Postgres>) -> Result<User> {
    match get_header(&request, "Authorization") {
        Some(auth_header) => {
            let (username, password) = parse_basic_auth(auth_header).await?;

            if username.is_empty() || password.is_empty() {
                return Err(GitError(401, Some("Incorrect username or password".to_owned())).into());
            }

            let option: Option<User> = sqlx::query_as::<_, User>("select * from users where username = $1 limit 1")
                .bind(&username)
                .fetch_optional(transaction)
                .await?;

            if option.is_none() {
                return Err(GitError(401, Some("Incorrect username or password".to_owned())).into());
            }

            let user = option.unwrap();

            if !crypto::check_password(&user, &password)? {
                return Err(GitError(401, Some("Incorrect username or password".to_owned())).into());
            }

            /*if user.disabled || verification::is_pending(&user, transaction).await? {
                return Err(GitError(401, Some("Account has been disabled".to_owned())).into());
            }*/

            Ok(user)
        }
        None => {
            Err(GitError(401, None).into())
        }
    }
}

pub(crate) async fn parse_basic_auth(auth_header: &str) -> Result<(String, String)> {
    let mut split = auth_header.splitn(2, " ");
    let auth_type = split.next().unwrap_or_default();
    let base64_creds = split.next().unwrap_or_default();

    if auth_type != "Basic" {
        return Err(GitError(401, None).into());
    }

    let creds = String::from_utf8(base64::decode(base64_creds)?)?;
    let mut splitted_creds = creds.splitn(2, ":");

    let username = splitted_creds.next().unwrap_or_default();
    let password = splitted_creds.next().unwrap_or_default();

    if username.is_empty() || password.is_empty() {
        return Err(GitError(401, Some("Incorrect username or password".to_owned())).into());
    }

    Ok((username.to_owned(), password.to_owned()))
}

pub(crate) async fn is_present(request: &HttpRequest) -> bool {
    get_header(&request, "Authorization").is_some()
}
