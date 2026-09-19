use crate::token::TokenPermission;
use crate::user::WebUser;

use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use gitarena_macros::route;

#[utoipa::path(
    get,
    path = "/api/tokens/permissions",
    responses(
        (status = 200, description = "Permissions that can be granted to a token", body = Vec<String>),
        (status = 401, description = "Authentication required"),
    ),
    security(("cookieAuth" = [])),
    tag = "token"
)]
#[route("/api/tokens/permissions", method = "GET", err = "json")]
pub(crate) async fn get_token_permissions(web_user: WebUser) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    Ok(HttpResponse::Ok().json(TokenPermission::grantable_permissions(user.admin)))
}
