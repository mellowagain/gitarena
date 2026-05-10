use crate::die;
use crate::organization::{OrgRole, Organization};
use crate::user::WebUser;
use crate::utils::identifiers::{is_namespace_taken, is_reserved_namespace, is_valid};

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_common::database::Pool;
use gitarena_macros::route;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub(crate) struct CreateOrgRequest {
    /// Name
    #[schema(max_length = 32, pattern = "^[a-z0-9_-]+$")]
    name: String,
    /// Description
    #[serde(default)]
    description: String,
}

#[utoipa::path(
    post,
    path = "/api/orgs",
    request_body = CreateOrgRequest,
    responses(
        (status = 200, description = "Organization created successfully", body = Organization),
        (status = 400, description = "Invalid organization name"),
        (status = 401, description = "Authentication required"),
        (status = 409, description = "Name already in use"),
    ),
    security(("cookieAuth" = [])),
    tag = "organization"
)]
#[route("/api/orgs", method = "POST", err = "json")]
pub(crate) async fn create_org(web_user: WebUser, body: web::Json<CreateOrgRequest>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let name = &body.name;

    if name.is_empty() || name.len() > 32 || !name.chars().all(is_valid) {
        die!(
            BAD_REQUEST,
            "Organization name must be between 1 and 32 characters long and may only contain a-z, 0-9, _ or -"
        );
    }

    if is_reserved_namespace(name) {
        die!(BAD_REQUEST, "Organization name is a reserved identifier");
    }

    let mut tx = db_pool.begin().await?;

    if is_namespace_taken(name, &mut tx).await? {
        die!(CONFLICT, "Organization name is already taken");
    }

    let description = &body.description;

    if description.len() > 256 {
        die!(BAD_REQUEST, "Description may only be up to 256 characters long");
    }

    let org = sqlx::query_as::<_, Organization>("insert into organizations (id, name, description) values ($1, $2, $3) returning *")
        .bind(Uuid::now_v7())
        .bind(name)
        .bind(description)
        .fetch_one(&mut *tx)
        .await?;

    sqlx::query("insert into organization_members (org_id, user_id, role) values ($1, $2, $3)")
        .bind(org.id)
        .bind(user.id)
        .bind(OrgRole::Owner)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(org))
}
