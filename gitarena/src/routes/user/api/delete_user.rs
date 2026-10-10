use crate::database::Pool;
use crate::meili::MeiliClient;
use crate::user::WebUser;
use crate::user::delete::delete_user;
use actix_identity::Identity;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use fang::AsyncQueue;
use gitarena_macros::route;

#[utoipa::path(
    delete,
    path = "/api/users/me",
    responses(
        (status = 204, description = "Account deleted"),
        (status = 401, description = "Authentication required"),
        (status = 409, description = "User is the last admin or the sole owner of an organization"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/users/me", method = "DELETE", err = "json")]
pub(crate) async fn delete_self(
    id: Identity,
    web_user: WebUser,
    request: HttpRequest,
    meili_client: web::Data<MeiliClient>,
    db_pool: web::Data<Pool>,
    queue: web::Data<AsyncQueue>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let tx = db_pool.begin().await?;

    delete_user(&user, user.id, &request, tx, &meili_client, &db_pool, &queue).await?;

    id.forget();

    Ok(HttpResponse::NoContent().finish())
}
