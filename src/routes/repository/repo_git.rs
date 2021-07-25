use crate::config::CONFIG;
use crate::error::GAErrors::GitError;
use crate::repository::Repository;

use std::borrow::Borrow;
use std::convert::TryFrom;
use std::path::Path;
use std::time::Duration;

use actix_web::http::header::HeaderName;
use actix_web::http::StatusCode;
use actix_web::web::Buf;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use anyhow::{Context, Result};
use futures::StreamExt;
use gitarena_macros::route;
use log::error;
use qstring::QString;
use serde::Deserialize;
use sqlx::PgPool;
use subprocess::{Exec, ExitStatus, Redirection};

#[route("/{username}/{repository}.git/info/refs", method="GET")]
pub(crate) async fn info_refs(uri: web::Path<GitRequest>, mut body: web::Payload, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let query_string = QString::from(request.query_string());

    let service = match query_string.get("service") {
        Some(value) => value,
        None => return Err(GitError(400, None).into())
    };

    if service != "git-upload-pack" {
        return Err(GitError(400, None).into());
    }

    let mut transaction = db_pool.begin().await?;

    let user_option: Option<(i32,)> = sqlx::query_as("select id from users where lower(username) = lower($1)")
        .bind(&uri.username)
        .fetch_optional(&mut transaction)
        .await?;

    let (user_id,) = match user_option {
        Some(user_id) => user_id,
        None => return Err(GitError(404, None).into())
    };

    let repo_option: Option<Repository> = sqlx::query_as::<_, Repository>("select * from repositories where owner = $1 and lower(name) = lower($2)")
        .bind(user_id)
        .bind(&uri.repository)
        .fetch_optional(&mut transaction)
        .await?;

    let _repo = match repo_option {
        Some(repo) => repo,
        None => return Err(GitError(404, None).into())
    };

    /* Preparation for allowing write over HTTPS
    if repo.private {
        if let Some(auth) = get_header(&request, "Authorization") {
            let mut split = auth.splitn(2, " ");
            let auth_type = split.next().unwrap_or_default();
            let base64_creds = split.next().unwrap_or_default();

            if auth_type != "Basic" {
                return Err(GitError(401, Some("Incorrect username or password".to_owned())).into());
            }

            let creds = String::from_utf8(base64::decode(base64_creds)?)?;
            let mut splitted_creds = creds.splitn(2, ":");

            let username = splitted_creds.next().unwrap_or_default();
            let password = splitted_creds.next().unwrap_or_default();

            if username.is_empty() || password.is_empty() {
                return Err(GitError(401, Some("Incorrect username or password".to_owned())).into());
            }

            let option: Option<User> = sqlx::query_as::<_, User>("select * from users where username = $1 limit 1")
                .bind(username)
                .fetch_optional(&mut transaction)
                .await?;

            if option.is_none() {
                return Err(GitError(401, Some("Incorrect username or password".to_owned())).into());
            }

            let user = option.unwrap();

            if !crypto::check_password(&user, &password.to_owned())? {
                return Err(GitError(401, Some("Incorrect username or password".to_owned())).into());
            }

            if user.disabled || verification::is_pending(&user, &mut transaction).await? {
                return Err(GitError(401, Some("Account has been disabled".to_owned())).into());
            }

            // Check additionally for other people with read access
            if user.id != repo.owner {
                return Err(GitError(404, None).into());
            }
        } else {
            return Ok(HttpResponse::Unauthorized()
                .header("WWW-Authenticate", "Basic realm=\"GitArena\", charset=\"UTF-8\"")
                .finish());
        }
    }*/

    let repo_base_dir_str: &str = CONFIG.repositories.base_dir.borrow();
    let repo_base_dir_path = Path::new(repo_base_dir_str);

    let mut bytes = web::BytesMut::new();

    while let Some(item) = body.next().await {
        let item = item?;
        bytes.extend_from_slice(&item);
    }

    let frozen_bytes = bytes.freeze();
    let body = String::from_utf8_lossy(frozen_bytes.bytes());

    let connection_info = request.connection_info();
    let request_path = request.uri().path().replace(".git", "");

    let mut popen = Exec::cmd("git")
        .arg("http-backend")
        .cwd(repo_base_dir_path)
        .env("GIT_PROJECT_ROOT", ".")
        .env("PATH_INFO", request_path)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("REMOTE_ADDR", connection_info.remote_addr().unwrap_or_default())
        .env("CONTENT_TYPE", request.content_type())
        .env("QUERY_STRING", request.query_string())
        .env("REQUEST_METHOD", request.method().as_str())
        .stdin(Redirection::Pipe)
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Pipe)
        .popen()
        .context("Failed to execute git subprocess")?;

    let (stdout_option, stderr_option) = popen.communicate(Some(body.borrow()))?;
    let stdout = stdout_option.unwrap_or_default();
    let stderr = stderr_option.unwrap_or_default();

    if let Some(status) = popen.wait_timeout(Duration::from_secs(30))? {
        match status {
            ExitStatus::Exited(status_code) => {
                if status_code != 0 {
                    error!("Git subprocess exited with non-zero ({}) status code: {}", status_code, stderr);
                }
            },
            _ => return Err(GitError(500, None).into())
        }
    } else {
        popen.kill()?;
        popen.wait()?;

        error!("Git subprocess timed out after 30 seconds");

        return Ok(HttpResponse::GatewayTimeout()
            .finish());
    }

    let mut response = HttpResponse::Ok();

    let lines = stdout.lines();
    let mut output_lines = Vec::<&str>::new();

    for line in lines {
        if line.is_empty() {
            continue;
        }

        let mut split = line.splitn(2, ":");
        let key = match split.next() {
            Some(key) => key,
            None => {
                continue
            }
        }.trim();

        if let Some(value) = split.next() {
            let trimmed_value = value.trim();

            if key == "Status" {
                match &trimmed_value[..3].parse::<u16>() {
                    Ok(status_code_num) => {
                        let status_code = StatusCode::from_u16(*status_code_num).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                        response.status(status_code);
                    }
                    Err(err) => {
                        response.status(StatusCode::INTERNAL_SERVER_ERROR);

                        error!("Unable to parse status code from Git subprocess stdout \"{}\": {}", trimmed_value, err);
                    }
                };

                continue;
            }

            if HeaderName::try_from(key).is_ok() {
                response.header(key, trimmed_value);
                continue;
            }
        }

        output_lines.push(line);
    }

    transaction.commit().await?;

    Ok(response.body(output_lines.join("\n")))
}

#[derive(Deserialize)]
pub(crate) struct GitRequest {
    username: String,
    repository: String
}
