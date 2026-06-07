use crate::database::Pool;
use crate::die;
use crate::session::Session;
use crate::user::WebUser;

use crate::events::{Event, Subject};
use actix_identity::Identity;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use tracing::debug;

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    responses(
        (status = 204, description = "Logged out successfully"),
        (status = 401, description = "Not logged in"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/auth/logout", method = "POST", err = "json")]
pub(crate) async fn post_logout(web_user: WebUser, id: Identity, request: HttpRequest, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    if matches!(web_user, WebUser::Anonymous) {
        die!(UNAUTHORIZED, "Not logged in");
    }

    let mut transaction = db_pool.begin().await?;

    if let Some(session) = Session::from_identity(id.identity(), &mut transaction).await? {
        debug!(user.id = %session.user_id, "User logged out");

        session.destroy(&mut transaction).await?;

        Event::new("auth.logout", session.user_id, &request, Subject::User(session.user_id), None)
            .save(&mut transaction)
            .await?;
    }

    id.forget();

    transaction.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}
