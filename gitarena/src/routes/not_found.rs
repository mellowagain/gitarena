use actix_web::{HttpRequest, HttpResponse, Responder};
use serde_json::json;
use tracing::debug;
use tracing::instrument;

#[instrument(skip_all)]
pub(crate) async fn default_handler(request: HttpRequest) -> actix_web::Result<impl Responder> {
    debug!(path = request.path(), "Got request for non-existent route");

    Ok(HttpResponse::NotFound().json(json!({
        "error": "Not found",
        "documentation": "https://git.mari.zip/docs/api-reference/introduction"
    })))
}
