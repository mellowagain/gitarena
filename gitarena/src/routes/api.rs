use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_common::database::Pool;
use gitarena_macros::{from_config, route};
use serde::Serialize;
use utoipa::ToSchema;

/// General GitArena information
#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct ApiInfoResponse {
    /// Application name
    app: &'static str,
    /// Version
    version: &'static str,
    /// Base url of this instance
    base_url: String,
    /// Link to the API documentation
    documentation: &'static str,
    /// Source repository URL
    repository: &'static str,
    /// Short git commit SHA of the running build
    commit: &'static str,
    /// Port of the SSH server, if running
    ssh_port: Option<i32>,
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
pub(crate) async fn api(db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let (domain, ssh_enabled, ssh_port) = from_config!(
        "domain" => String,
        "ssh.enabled" => bool,
        "ssh.port" => i32
    );

    Ok(HttpResponse::Ok().json(ApiInfoResponse {
        app: "GitArena",
        version: env!("CARGO_PKG_VERSION"),
        base_url: domain,
        documentation: "https://git.mari.zip/rapidoc",
        repository: env!("CARGO_PKG_REPOSITORY"),
        commit: env!("VERGEN_GIT_SHA"),
        ssh_port: if ssh_enabled { Some(ssh_port) } else { None },
    }))
}
