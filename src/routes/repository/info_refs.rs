use crate::error::GAErrors::GitError;
use crate::git::basic_auth;
use crate::git::capabilities::capabilities;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use qstring::QString;
use sqlx::PgPool;

#[route("/{username}/{repository}.git/info/refs", method="GET")]
pub(crate) async fn info_refs(uri: web::Path<GitRequest>, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
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

    let is_none = repo_option.is_none();

    // Prompt for authentication even if the repo does not exist to prevent leakage of private repositories
    if is_none || repo_option.unwrap().private {
        if !basic_auth::is_present(&request).await {
            return Ok(HttpResponse::Unauthorized()
                .header("WWW-Authenticate", "Basic realm=\"GitArena\", charset=\"UTF-8\"")
                .finish());
        }

        let user = basic_auth::authenticate(&request, &mut transaction).await?;

        if is_none {
            return Err(GitError(404, None).into());
        }

        // Check if the user has access rights to the repository
        // TODO: Check for collaborators, currently only checks for owner
        if user.username.to_lowercase() != uri.username.to_lowercase() {
            return Err(GitError(404, None).into());
        }
    }

    transaction.commit().await?;

    Ok(HttpResponse::Ok()
        .header("Cache-Control", "no-cache, max-age=0, must-revalidate")
        .header("Content-Type", "application/x-git-upload-pack-advertisement")
        .header("Expires", "Fri, 01 Jan 1980 00:00:00 GMT")
        .header("Pragma", "no-cache")
        .body(capabilities(service).await?))
}
