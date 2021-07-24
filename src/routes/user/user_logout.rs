use crate::error::GAErrors::HttpError;

use actix_identity::Identity;
use actix_web::http::header;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use sqlx::PgPool;

#[route("/logout", method="POST")]
pub(crate) async fn logout(id: Identity) -> Result<impl Responder> {
    if id.identity().is_none() {
        // Maybe just redirect to home page?
        return Err(HttpError(401, "Already logged out".to_owned()).into());
    }

    id.forget();

    Ok(HttpResponse::Found()
        .header(header::LOCATION, "/")
        .finish())
}
