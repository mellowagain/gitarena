use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use gitarena_macros::route;
use serde::Serialize;
use utoipa::ToSchema;

/// General GitArena information
#[derive(Serialize, ToSchema)]
pub(crate) struct ApiInfoResponse {
    /// Application name
    app: &'static str,
    /// Version
    version: &'static str,
    /// Link to the API documentation
    documentation: &'static str,
    /// Source repository URL
    repository: &'static str,
    /// Short git commit SHA of the running build
    commit: &'static str,
}

#[utoipa::path(
    get,
    path = "/api",
    responses(
        (status = 200, description = "API information", body = ApiInfoResponse),
    ),
    tag = "api"
)]
#[route("/api", method = "GET", err = "json")]
pub(crate) async fn api() -> Result<impl Responder> {
    Ok(HttpResponse::Ok().json(ApiInfoResponse {
        app: "GitArena",
        version: env!("CARGO_PKG_VERSION"),
        documentation: "https://git.mari.zip/rapidoc",
        repository: env!("CARGO_PKG_REPOSITORY"),
        commit: env!("VERGEN_GIT_SHA"),
    }))
}
