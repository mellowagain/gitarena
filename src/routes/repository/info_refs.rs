use crate::error::GAErrors::GitError;
use crate::extensions::get_header;
use crate::git::basic_auth;
use crate::git::capabilities::capabilities;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;

use actix_web::{Either, HttpRequest, HttpResponse, Responder, web};
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

    let git_protocol = get_header(&request, "Git-Protocol").unwrap_or_default();

    if git_protocol != "version=2" {
        return Err(GitError(400, Some("Unsupported Git protocol version".to_owned())).into());
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

    let (_, _) = match basic_auth::validate_repo_access(repo_option,"application/x-git-upload-pack-advertisement", &request, &mut transaction).await? {
        Either::A(tuple) => tuple,
        Either::B(response) => return Ok(response)
    };

    transaction.commit().await?;

    Ok(HttpResponse::Ok()
        .header("Cache-Control", "no-cache, max-age=0, must-revalidate")
        .header("Content-Type", "application/x-git-upload-pack-advertisement")
        .body(capabilities(service).await?))
}
