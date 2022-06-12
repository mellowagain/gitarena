use crate::die;
use crate::prelude::HttpRequestExtensions;
use crate::session::Session;

use actix_identity::Identity;
use actix_web::http::header::LOCATION;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use log::debug;
use sqlx::PgPool;

#[route("/logout", method = "POST", err = "htmx+html")]
pub(crate) async fn logout(request: HttpRequest, id: Identity, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    if id.identity().is_none() {
        // Maybe just redirect to home page?
        die!(UNAUTHORIZED, "Already logged out");
    }

    let mut transaction = db_pool.begin().await?;

    if let Some(session) = Session::from_identity(id.identity(), &mut transaction).await.ok().flatten() {
        debug!("Destroying a session for user id {}", &session.user_id);

        session.destroy(&mut transaction).await?;
    }

    id.forget();

    transaction.commit().await?;

    Ok(if request.is_htmx() {
        HttpResponse::Ok().append_header(("hx-redirect", "/")).append_header(("hx-refresh", "true")).finish()
    } else {
        HttpResponse::Found().append_header((LOCATION, "/")).finish()
    })
}
