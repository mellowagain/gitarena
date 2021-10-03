use crate::error::GitArenaError;
use crate::extensions::get_user_by_identity;
use crate::render_template;

use actix_identity::Identity;
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
        "error": "Not found"
    })))
}

async fn web_not_found(request: HttpRequest, id: Identity, db_pool: web::Data<PgPool>) -> Result<HttpResponse> {
    let mut transaction = db_pool.begin().await?;
    let mut context = Context::new();

    if let Some(user) = get_user_by_identity(id.identity(), &mut transaction).await {
        context.try_insert("user", &user)?;
    }

    context.try_insert("path", request.path())?;

    render_template!(StatusCode::NOT_FOUND, "error/404.html", context, transaction)
}

#[instrument(skip_all)]
pub(crate) async fn default_handler(request: HttpRequest, id: Identity, db_pool: web::Data<PgPool>) -> ActixResult<impl Responder> {
    debug!("Got request for non-existent resource: {}", request.path());

    Ok(if !request.path().starts_with("/api") {
        web_not_found(request, id, db_pool).await
    } else {
        api_not_found().await
    }.map_err(|err| -> GitArenaError { err.into() }))
}
