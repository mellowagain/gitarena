use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use gitarena_macros::route;
use serde_json::json;

#[route("/api", method = "GET", err = "json")]
pub(crate) async fn api() -> Result<impl Responder> {
    Ok(HttpResponse::Ok().json(json!({
        "app": "GitArena",
        "version": env!("CARGO_PKG_VERSION"),
        "documentation": "https://gitarena.com/docs/api",
        "repository": env!("CARGO_PKG_REPOSITORY"),
        "commit": env!("VERGEN_GIT_SHA")
    })))
}
