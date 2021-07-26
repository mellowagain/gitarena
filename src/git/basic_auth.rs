use crate::crypto;
use crate::error::GAErrors::GitError;
use crate::extensions::get_header;
use crate::user::User;

use actix_web::HttpRequest;
use anyhow::Result;
use sqlx::{Postgres, Transaction};

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
