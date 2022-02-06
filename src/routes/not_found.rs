use crate::error::{ErrorDisplayType, GitArenaError};
use crate::prelude::ContextExtensions;
use crate::render_template;
use crate::user::WebUser;

use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::Result as ActixResult;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use log::debug;
use serde_json::json;
use sqlx::PgPool;
use tera::Context;
use tracing::instrument;

async fn api_not_found() -> Result<HttpResponse> {
    Ok(HttpResponse::NotFound().json(json!({
        "error": "Not found",
        "documentation": "https://gitarena.com/docs/api"
    })))
}

async fn web_not_found(request: HttpRequest, web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<HttpResponse> {
    let mut transaction = db_pool.begin().await?;
    let mut context = Context::new();

    context.insert_web_user(&web_user)?;
    context.try_insert("path", request.path())?;

    render_template!(StatusCode::NOT_FOUND, "error/404.html", context, transaction)
}

#[instrument(skip_all)]
pub(crate) async fn default_handler(request: HttpRequest, web_user: WebUser, db_pool: web::Data<PgPool>) -> ActixResult<impl Responder> {
    debug!("Got request for non-existent resource: {}", request.path());

    Ok(if !request.path().starts_with("/api") {
        web_not_found(request, web_user, db_pool).await.map_err(|err| GitArenaError {
            source: Arc::new(err),
            display_type: ErrorDisplayType::Html
        })
    } else {
        api_not_found().await.map_err(|err| GitArenaError {
            source: Arc::new(err),
            display_type: ErrorDisplayType::Json
        })
    })
}
