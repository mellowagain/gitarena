use crate::die;
use crate::organization::{OrgMember, OrgRole, Organization};
use crate::user::WebUser;

use crate::database::Pool;
use crate::events::Event;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde::Deserialize;
use serde_json::json;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/api/orgs/{name}",
    params(
        ("name" = String, Path, description = "Organization name")
    ),
    responses(
        (status = 200, description = "Organization info", body = Organization),
        (status = 404, description = "Organization not found"),
    ),
    tag = "organization"
)]
#[route("/api/orgs/{name}", method = "GET", err = "json")]
pub(crate) async fn get_org(name: web::Path<String>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut tx = db_pool.begin().await?;

    let org = Organization::find_by_name(&name, &mut tx).await;

    tx.commit().await?;

    match org {
        Some(o) => Ok(HttpResponse::Ok().json(o)),
        None => die!(NOT_FOUND, "Organization not found"),
    }
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{name}",
    params(
        ("name" = String, Path, description = "Organization name")
    ),
    responses(
        (status = 204, description = "Organization deleted"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Organization not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "organization"
)]
#[route("/api/orgs/{name}", method = "DELETE", err = "json")]
pub(crate) async fn delete_org(web_user: WebUser, name: web::Path<String>, request: HttpRequest, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut tx = db_pool.begin().await?;

    let Some(org) = Organization::find_by_name(&name, &mut tx).await else {
        die!(NOT_FOUND, "Organization not found");
    };

    let role = OrgMember::get_role(org.id, user.id, &mut tx).await?;

    if !role.is_some_and(|r| OrgMember::has_permission(r, OrgRole::Admin)) {
        die!(FORBIDDEN, "Insufficient permissions to delete this organization");
    }

    Event::new(
        "org.deleted",
        user.id,
        &request,
        (&org).into(),
        Some(json!({
            "id": org.id,
            "name": org.name
        })),
    )
    .save(&mut tx)
    .await?;

    sqlx::query("delete from organizations where id = $1").bind(org.id).execute(&mut *tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct UpdateOrgRequest {
    description: String,
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{name}",
    params(
        ("name" = String, Path, description = "Organization name")
    ),
    request_body = UpdateOrgRequest,
    responses(
        (status = 204, description = "Organization updated"),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Organization not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "organization"
)]
#[route("/api/orgs/{name}", method = "PATCH", err = "json")]
pub(crate) async fn update_org(
    web_user: WebUser,
    name: web::Path<String>,
    body: web::Json<UpdateOrgRequest>,
    request: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if body.description.len() > 256 {
        die!(BAD_REQUEST, "Description may only be up to 256 characters long");
    }

    let mut tx = db_pool.begin().await?;

    let Some(org) = Organization::find_by_name(&name, &mut tx).await else {
        die!(NOT_FOUND, "Organization not found");
    };

    let role = OrgMember::get_role(org.id, user.id, &mut tx).await?;

    if !role.is_some_and(|r| OrgMember::has_permission(r, OrgRole::Admin)) {
        die!(FORBIDDEN, "Insufficient permissions to update this organization");
    }

    sqlx::query("update organizations set description = $1 where id = $2")
        .bind(&body.description)
        .bind(org.id)
        .execute(&mut *tx)
        .await?;

    Event::new(
        "org.updated",
        user.id,
        &request,
        (&org).into(),
        Some(json!({
            "fields": ["description"],
            "description": body.description
        })),
    )
    .save(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}
