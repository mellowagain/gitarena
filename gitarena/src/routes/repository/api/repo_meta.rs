use crate::repository::Repository;

use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use gitarena_macros::route;

#[route("/api/repo/{username}/{repository}", method = "GET", err = "json")]
pub(crate) async fn meta(repo: Repository) -> Result<impl Responder> {
    Ok(HttpResponse::Ok().json(repo))
}
