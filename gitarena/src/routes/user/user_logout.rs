use crate::database::Pool;
use crate::die;
use crate::session::Session;

use actix_identity::Identity;
use actix_web::http::header::LOCATION;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use tracing::debug;

#[route("/logout", method = "POST", err = "text")]
pub(crate) async fn logout(_request: HttpRequest, id: Identity, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    if id.id().is_err() {
        // Maybe just redirect to home page?
        die!(UNAUTHORIZED, "Already logged out");
    }

    let mut transaction = db_pool.begin().await?;

    if let Some(session) = Session::from_identity(id.id().ok(), &mut transaction).await.ok().flatten() {
        debug!(user.id = %session.user_id, "User logged out");
        session.destroy(&mut transaction).await?;
    }

    id.logout();

    transaction.commit().await?;

    Ok(HttpResponse::Found().append_header((LOCATION, "/")).finish())
}
