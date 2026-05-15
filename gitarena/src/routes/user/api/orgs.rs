use crate::err;
use crate::user::User;

use crate::database::Pool;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema, FromRow)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserOrgEntry {
    /// Organization ID
    id: Uuid,
    /// Organization name
    name: String,
}

#[utoipa::path(
    get,
    path = "/api/users/{username}/orgs",
    params(
        ("username" = String, Path, description = "Username"),
    ),
    responses(
        (status = 200, description = "List of organizations the user is a member of", body = Vec<UserOrgEntry>),
        (status = 404, description = "User not found"),
    ),
    tag = "user"
)]
#[route("/api/users/{username}/orgs", method = "GET", err = "json")]
pub(crate) async fn get_user_orgs(path: web::Path<String>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let username = path.into_inner();
    let mut tx = db_pool.begin().await?;

    let user = User::find_using_name(&username, &mut tx)
        .await
        .ok_or_else(|| err!(NOT_FOUND, "User not found"))?;

    let orgs: Vec<UserOrgEntry> = sqlx::query_as(
        "select organizations.id, organizations.name \
         from organization_members \
         join organizations on organizations.id = organization_members.org_id \
         where organization_members.user_id = $1 \
         order by organizations.name",
    )
    .bind(user.id)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(orgs))
}
