use crate::database::Pool;
use crate::die;
use crate::events::Event;
use crate::routes::tokens::{can_administer, find_token, subject_of};
use crate::token::Token;
use crate::user::WebUser;

use actix_web::web::{Data, Path};
use actix_web::{HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use gitarena_macros::route;
use serde_json::json;
use uuid::Uuid;

#[utoipa::path(
    delete,
    path = "/api/tokens/{id}",
    params(("id" = Uuid, Path, description = "Token ID")),
    responses(
        (status = 204, description = "Token revoked"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Token not found"),
        (status = 409, description = "Token is already revoked"),
    ),
    security(("cookieAuth" = [])),
    tag = "token"
)]
#[route("/api/tokens/{id}", method = "DELETE", err = "json")]
pub(crate) async fn revoke_token(path: Path<Uuid>, web_user: WebUser, request: HttpRequest, db_pool: Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut tx = db_pool.begin().await?;

    let Some(existing) = find_token(path.into_inner(), &mut tx).await? else {
        die!(NOT_FOUND, "Token not found");
    };

    if !can_administer(existing.token.owner_user, existing.token.owner_org, existing.token.owner_repo, &user, &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions to revoke this token");
    }

    if existing.token.revoked_at.is_some() {
        die!(CONFLICT, "Token is already revoked");
    }

    let token = sqlx::query_as::<_, Token>("update tokens set revoked_at = now() where id = $1 returning *")
        .bind(existing.token.id)
        .fetch_one(&mut *tx)
        .await?;

    Event::new(
        "token.revoked",
        user.id,
        &request,
        subject_of(&token, &user),
        Some(json!({
            "token": token.id,
            "name": token.name,
            "type": token.token_type
        })),
    )
    .save(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}
