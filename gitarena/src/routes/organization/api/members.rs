use crate::die;
use crate::organization::{OrgMember, OrgRole, Organization};
use crate::user::{User, WebUser};

use crate::database::Pool;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema, FromRow)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OrgMemberEntry {
    /// User ID
    user_id: Uuid,
    /// Role
    role: OrgRole,
}

#[utoipa::path(
    get,
    path = "/api/orgs/{name}/members",
    params(
        ("name" = String, Path, description = "Organization name")
    ),
    responses(
        (status = 200, description = "List of members", body = Vec<OrgMemberEntry>),
        (status = 404, description = "Organization not found"),
    ),
    tag = "organization"
)]
#[route("/api/orgs/{name}/members", method = "GET", err = "json")]
pub(crate) async fn list_members(name: web::Path<String>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut tx = db_pool.begin().await?;

    let Some(org) = Organization::find_by_name(&name, &mut tx).await else {
        die!(NOT_FOUND, "Organization not found");
    };

    let members = sqlx::query_as::<_, OrgMemberEntry>("select user_id, role from organization_members where org_id = $1 order by role")
        .bind(org.id)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(members))
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct AddMemberRequest {
    /// Username of the user to add
    username: String,
    /// Role to assign
    #[serde(default = "default_role")]
    role: OrgRole,
}

fn default_role() -> OrgRole {
    OrgRole::Member
}

#[utoipa::path(
    put,
    path = "/api/orgs/{name}/members",
    params(
        ("name" = String, Path, description = "Organization name")
    ),
    request_body = AddMemberRequest,
    responses(
        (status = 204, description = "Member added or updated"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Organization or user not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "organization"
)]
#[route("/api/orgs/{name}/members", method = "PUT", err = "json")]
pub(crate) async fn add_member(
    web_user: WebUser,
    name: web::Path<String>,
    body: web::Json<AddMemberRequest>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let actor = web_user.into_user()?;

    let mut tx = db_pool.begin().await?;

    let Some(org) = Organization::find_by_name(&name, &mut tx).await else {
        die!(NOT_FOUND, "Organization not found");
    };

    let actor_role = OrgMember::get_role(org.id, actor.id, &mut tx).await?;

    if !actor_role.is_some_and(|role| OrgMember::has_permission(role, OrgRole::Admin)) {
        die!(FORBIDDEN, "Insufficient permissions to manage members");
    }

    let Some(target) = User::find_using_name(&body.username, &mut tx).await else {
        die!(NOT_FOUND, "User not found");
    };

    // cannot demote another owner unless you are also owner
    if let Some(existing_role) = OrgMember::get_role(org.id, target.id, &mut tx).await?
        && existing_role == OrgRole::Owner
        && actor_role.is_none_or(|ar| !OrgMember::has_permission(ar, OrgRole::Owner))
    {
        die!(FORBIDDEN, "Only an owner can change another owners role");
    }

    sqlx::query(
        "insert into organization_members (org_id, user_id, role) values ($1, $2, $3) on conflict (org_id, user_id) do update set role = excluded.role",
    )
    .bind(org.id)
    .bind(target.id)
    .bind(body.role)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{name}/members/{username}",
    params(
        ("name" = String, Path, description = "Organization name"),
        ("username" = String, Path, description = "Username to remove"),
    ),
    responses(
        (status = 204, description = "Member removed"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Organization or user not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "organization"
)]
#[route("/api/orgs/{name}/members/{username}", method = "DELETE", err = "json")]
pub(crate) async fn remove_member(web_user: WebUser, path: web::Path<(String, String)>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let actor = web_user.into_user()?;
    let (org_name, target_name) = path.into_inner();

    let mut tx = db_pool.begin().await?;

    let Some(org) = Organization::find_by_name(&org_name, &mut tx).await else {
        die!(NOT_FOUND, "Organization not found");
    };

    let actor_role = OrgMember::get_role(org.id, actor.id, &mut tx).await?;

    let Some(target) = User::find_using_name(&target_name, &mut tx).await else {
        die!(NOT_FOUND, "User not found");
    };

    // allow users to remove themselves, otherwise require admin
    if target.id != actor.id && !actor_role.is_some_and(|role| OrgMember::has_permission(role, OrgRole::Admin)) {
        die!(FORBIDDEN, "Insufficient permissions to remove members");
    }

    let target_role = OrgMember::get_role(org.id, target.id, &mut tx).await?;

    if target_role == Some(OrgRole::Owner) {
        let (owner_count,): (i64,) = sqlx::query_as("select count(*) from organization_members where org_id = $1 and role = 'owner'")
            .bind(org.id)
            .fetch_one(&mut *tx)
            .await?;

        if owner_count <= 1 {
            die!(BAD_REQUEST, "Cannot remove the last owner of an organization");
        }
    }

    sqlx::query("delete from organization_members where org_id = $1 and user_id = $2")
        .bind(org.id)
        .bind(target.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}
