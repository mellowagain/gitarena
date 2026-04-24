use crate::die;
use crate::user::WebUser;

use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use gitarena_macros::route;
use serde::Serialize;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/api/auth/me",
    responses(
        (status = 200, description = "Currently authenticated user", body = MeResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/auth/me", method = "GET", err = "json")]
pub(crate) async fn get_me(web_user: WebUser) -> Result<impl Responder> {
    let user = match web_user {
        WebUser::Authenticated(user) => user,
        WebUser::Anonymous => die!(UNAUTHORIZED, "Not authenticated"),
    };

    Ok(HttpResponse::Ok().json(MeResponse {
        id: user.id,
        username: user.username,
        admin: user.admin,
    }))
}

#[derive(Serialize, ToSchema)]
pub(crate) struct MeResponse {
    pub(crate) id: i32,
    pub(crate) username: String,
    pub(crate) admin: bool,
}
