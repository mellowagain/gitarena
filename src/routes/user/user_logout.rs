use crate::error::GAErrors::HttpError;
use crate::prelude::*;
use crate::session::Session;

use actix_identity::Identity;
use actix_web::http::header::LOCATION;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use log::debug;
use sqlx::PgPool;

#[route("/logout", method = "POST")]
pub(crate) async fn logout(request: HttpRequest, id: Identity, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    if id.identity().is_none() {
        // Maybe just redirect to home page?
        return Err(HttpError(401, "Already logged out".to_owned()).into());
    }

    let mut transaction = db_pool.begin().await?;

    if let Some(session) = Session::from_identity(id.identity(), &mut transaction).await.ok().flatten() {
        debug!("Destroying session id {}", session.id);

        session.destroy(&mut transaction).await?;
    }

    id.forget();

    transaction.commit().await?;

    Ok(if request.get_header("hx-request").is_some() {
        HttpResponse::Ok().header("hx-redirect", "/").header("hx-refresh", "true").finish()
    } else {
        HttpResponse::Found().header(LOCATION, "/").finish()
    })
}
