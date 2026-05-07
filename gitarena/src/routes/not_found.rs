use crate::error::{ErrorDisplayType, GitArenaError};
use crate::prelude::ContextExtensions;
use crate::render_template;
use crate::user::WebUser;
use gitarena_common::database::Pool;

use std::sync::Arc;

use actix_web::Result as ActixResult;
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use serde_json::json;
use tracing::debug;

use tera::Context;
use tracing::instrument;

fn api_not_found() -> HttpResponse {
    HttpResponse::NotFound().json(json!({
        "error": "Not found",
        "documentation": "https://gitarena.com/docs/api"
    }))
}

async fn web_not_found(request: HttpRequest, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<HttpResponse> {
    let mut transaction = db_pool.begin().await?;
    let mut context = Context::new();

    context.insert_web_user(&web_user)?;
    context.try_insert("path", request.path())?;

    render_template!(StatusCode::NOT_FOUND, "error/404.html", context, transaction)
}

#[instrument(skip_all)]
pub(crate) async fn default_handler(request: HttpRequest, web_user: WebUser, db_pool: web::Data<Pool>) -> ActixResult<impl Responder> {
    debug!(path = request.path(), "Got request for non-existent route");

    Ok(if request.path().starts_with("/api") {
        Ok(api_not_found())
    } else {
        web_not_found(request, web_user, db_pool).await.map_err(|err| GitArenaError {
            source: Arc::new(err),
            display_type: ErrorDisplayType::Html,
        })
    })
}
