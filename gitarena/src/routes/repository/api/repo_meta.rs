use crate::repository::Repository;

use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use gitarena_macros::route;

#[utoipa::path(
    get,
    path = "/api/repo/{username}/{repository}",
    params(
        ("username" = String, Path, description = "Repository owner username"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "Repository metadata", body = Repository),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repo/{username}/{repository}", method = "GET", err = "json")]
pub(crate) async fn meta(repo: Repository) -> Result<impl Responder> {
    Ok(HttpResponse::Ok().json(repo))
}
