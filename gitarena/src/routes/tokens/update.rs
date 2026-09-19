use crate::database::Pool;
use crate::die;
use crate::events::Event;
use crate::routes::tokens::{
    TokenResponse, can_administer, check_permissions, check_scope_targets, find_token, insert_scope_targets, name_taken, scope_allowed, subject_of,
};
use crate::token::{Token, TokenPermission, TokenScope};
use crate::user::WebUser;

use actix_web::web::{Data, Json, Path};
use actix_web::{HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use gitarena_macros::route;
use serde::Deserialize;
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    patch,
    path = "/api/tokens/{id}",
    params(("id" = Uuid, Path, description = "Token ID")),
    request_body = UpdateTokenRequest,
    responses(
        (status = 200, description = "Token updated", body = TokenResponse),
        (status = 400, description = "Invalid name, scope or permissions"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Token not found"),
        (status = 409, description = "Token is revoked or a token with this name already exists"),
    ),
    security(("cookieAuth" = [])),
    tag = "token"
)]
#[route("/api/tokens/{id}", method = "PATCH", err = "json")]
pub(crate) async fn update_token(
    path: Path<Uuid>,
    web_user: WebUser,
    body: Json<UpdateTokenRequest>,
    request: HttpRequest,
    db_pool: Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let body = body.into_inner();

    let mut tx = db_pool.begin().await?;

    let Some(existing) = find_token(path.into_inner(), &mut tx).await? else {
        die!(NOT_FOUND, "Token not found");
    };

    if !can_administer(existing.token.owner_user, existing.token.owner_org, existing.token.owner_repo, &user, &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions to update this token");
    }

    if existing.token.revoked_at.is_some() {
        die!(CONFLICT, "Token is revoked");
    }

    let name = body.name.unwrap_or(existing.token.name);
    let permissions = body.permissions.unwrap_or_else(|| existing.token.permissions.clone());
    let scope = body.scope.unwrap_or(existing.token.scope);

    if name.is_empty() {
        die!(BAD_REQUEST, "Token requires a name");
    }

    if permissions.is_empty() {
        die!(BAD_REQUEST, "Token requires at least one permission");
    }

    check_permissions(&permissions, &existing.token.permissions, &user)?;

    if !scope_allowed(existing.token.token_type, scope) {
        die!(BAD_REQUEST, "Scope is not allowed for this token type");
    }

    let targets_touched = body.scope.is_some() || body.scope_orgs.is_some() || body.scope_repos.is_some();

    let inherit_targets = scope == TokenScope::Selected;
    let scope_orgs = body.scope_orgs.unwrap_or(if inherit_targets { existing.scope_orgs } else { Vec::new() });
    let scope_repos = body.scope_repos.unwrap_or(if inherit_targets { existing.scope_repos } else { Vec::new() });

    if targets_touched {
        check_scope_targets(
            existing.token.token_type,
            scope,
            existing.token.owner_org,
            &scope_orgs,
            &scope_repos,
            &user,
            &mut tx,
        )
        .await?;
    }

    let owner = (existing.token.owner_user, existing.token.owner_org, existing.token.owner_repo);

    if name_taken(&name, Some(existing.token.id), owner, &mut tx).await? {
        die!(CONFLICT, "A token with this name already exists");
    }

    let token = sqlx::query_as::<_, Token>("update tokens set name = $1, permissions = $2, scope = $3 where id = $4 returning *")
        .bind(&name)
        .bind(&permissions)
        .bind(scope)
        .bind(existing.token.id)
        .fetch_one(&mut *tx)
        .await?;

    if targets_touched {
        sqlx::query("delete from token_scopes where token_id = $1")
            .bind(token.id)
            .execute(&mut *tx)
            .await?;

        insert_scope_targets(token.id, token.token_type, &scope_orgs, &scope_repos, &mut tx).await?;
    }

    Event::new(
        "token.updated",
        user.id,
        &request,
        subject_of(&token, &user),
        Some(json!({
            "token": token.id,
            "name": token.name,
            "scope": scope,
            "permissions": permissions
        })),
    )
    .save(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(TokenResponse {
        token,
        scope_orgs,
        scope_repos,
    }))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateTokenRequest {
    /// New name
    #[schema(min_length = 1)]
    name: Option<String>,
    /// New permissions
    permissions: Option<Vec<TokenPermission>>,
    /// New scope
    scope: Option<TokenScope>,
    /// New UUIDs of the organizations to limit this token to
    scope_orgs: Option<Vec<Uuid>>,
    /// New UUIDs of the repositories to limit this token to
    scope_repos: Option<Vec<Uuid>>,
}
