use crate::database::Pool;
use crate::die;
use crate::events::Event;
use crate::routes::tokens::{
    TokenResponse, can_administer, check_permissions, check_scope_targets, insert_scope_targets, name_taken, scope_allowed, subject_of,
};
use crate::token::{Token, TokenPermission, TokenScope, TokenType};
use crate::user::WebUser;

use actix_web::web::{Data, Json};
use actix_web::{HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use chrono::serde::ts_seconds_option;
use chrono::{DateTime, Utc};
use gitarena_macros::{from_config, route};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/api/tokens",
    request_body = CreateTokenRequest,
    responses(
        (status = 201, description = "Token created", body = CreateTokenResponse),
        (status = 400, description = "Invalid owner, scope, permissions or expiration date"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 409, description = "A token with this name already exists"),
    ),
    security(("cookieAuth" = [])),
    tag = "token"
)]
#[route("/api/tokens", method = "POST", err = "json")]
pub(crate) async fn create_token(web_user: WebUser, body: Json<CreateTokenRequest>, request: HttpRequest, db_pool: Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let body = body.into_inner();

    if body.name.is_empty() {
        die!(BAD_REQUEST, "Token requires a name");
    }

    if body.permissions.is_empty() {
        die!(BAD_REQUEST, "Token requires at least one permission");
    }

    check_permissions(&body.permissions, &[], &user)?;

    if !scope_allowed(body.token_type, body.scope) {
        die!(BAD_REQUEST, "Scope is not allowed for this token type");
    }

    if let Some(expires_at) = body.expires_at
        && expires_at <= Utc::now()
    {
        die!(BAD_REQUEST, "Expiration date must be in the future");
    }

    let (owner_user, owner_org, owner_repo) = match body.token_type {
        TokenType::Personal => {
            if body.owner_user.is_some_and(|owner| owner != user.id) || body.owner_org.is_some() || body.owner_repo.is_some() {
                die!(BAD_REQUEST, "Personal tokens can only be owned by the creating user");
            }

            (Some(user.id), None, None)
        }
        TokenType::Organization => {
            let Some(org) = body.owner_org else {
                die!(BAD_REQUEST, "Organization tokens require an owning organization");
            };

            if body.owner_user.is_some() || body.owner_repo.is_some() {
                die!(BAD_REQUEST, "Organization tokens can only be owned by an organization");
            }

            (None, Some(org), None)
        }
        TokenType::Deploy => {
            let Some(repo) = body.owner_repo else {
                die!(BAD_REQUEST, "Deploy tokens require an owning repository");
            };

            if body.owner_user.is_some() || body.owner_org.is_some() {
                die!(BAD_REQUEST, "Deploy tokens can only be owned by a repository");
            }

            (None, None, Some(repo))
        }
        TokenType::Runner => match (body.owner_user, body.owner_org, body.owner_repo) {
            (Some(owner), None, None) => {
                if owner != user.id {
                    die!(BAD_REQUEST, "Runner tokens can only be owned by the creating user");
                }

                (Some(owner), None, None)
            }
            (None, Some(org), None) => (None, Some(org), None),
            (None, None, Some(repo)) => (None, None, Some(repo)),
            (None, None, None) => (None, None, None),
            _ => die!(BAD_REQUEST, "Runner tokens can only have a single owner"),
        },
        TokenType::Instance => {
            if body.owner_user.is_some() || body.owner_org.is_some() || body.owner_repo.is_some() {
                die!(BAD_REQUEST, "Instance tokens cannot be owned by anyone");
            }

            (None, None, None)
        }
        TokenType::Ci => die!(BAD_REQUEST, "CI tokens are issued automatically to CI workflows"),
    };

    let secret_key = from_config!("secret" => String);

    let mut tx = db_pool.begin().await?;

    if !can_administer(owner_user, owner_org, owner_repo, &user, &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions to create this token");
    }

    check_scope_targets(body.token_type, body.scope, owner_org, &body.scope_orgs, &body.scope_repos, &user, &mut tx).await?;

    if name_taken(&body.name, None, (owner_user, owner_org, owner_repo), &mut tx).await? {
        die!(CONFLICT, "A token with this name already exists");
    }

    let (secret, hmac) = Token::generate_secret(body.token_type, &secret_key)?;

    let token = sqlx::query_as::<_, Token>(
        "insert into tokens (id, name, token, owner_user, owner_org, owner_repo, creator, token_type, scope, permissions, expires_at) \
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) on conflict do nothing returning *",
    )
    .bind(Uuid::now_v7())
    .bind(&body.name)
    .bind(hmac)
    .bind(owner_user)
    .bind(owner_org)
    .bind(owner_repo)
    .bind(user.id)
    .bind(body.token_type)
    .bind(body.scope)
    .bind(&body.permissions)
    .bind(body.expires_at)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(token) = token else {
        die!(CONFLICT, "A token with this name already exists");
    };

    insert_scope_targets(token.id, body.token_type, &body.scope_orgs, &body.scope_repos, &mut tx).await?;

    Event::new(
        "token.created",
        user.id,
        &request,
        subject_of(&token, &user),
        Some(json!({
            "token": token.id,
            "name": token.name,
            "type": body.token_type,
            "scope": body.scope,
            "permissions": body.permissions
        })),
    )
    .save(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Created().json(CreateTokenResponse {
        token: TokenResponse {
            token,
            scope_orgs: body.scope_orgs,
            scope_repos: body.scope_repos,
        },
        secret,
    }))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateTokenRequest {
    /// Name
    #[schema(min_length = 1)]
    name: String,
    /// Type
    token_type: TokenType,
    /// Scope
    #[serde(default = "default_scope")]
    scope: TokenScope,
    /// Permissions
    permissions: Vec<TokenPermission>,
    /// Expiration date as a Unix timestamp (seconds since epoch)
    #[serde(default, with = "ts_seconds_option")]
    expires_at: Option<DateTime<Utc>>,
    /// UUID of the user that should own this token. Can only ever be the creating user
    owner_user: Option<Uuid>,
    /// UUID of the organization that should own this token
    owner_org: Option<Uuid>,
    /// UUID of the repository that should own this token
    owner_repo: Option<Uuid>,
    /// UUIDs of the organizations to limit this token to
    #[serde(default)]
    scope_orgs: Vec<Uuid>,
    /// UUIDs of the repositories to limit this token to
    #[serde(default)]
    scope_repos: Vec<Uuid>,
}

fn default_scope() -> TokenScope {
    TokenScope::All
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateTokenResponse {
    token: TokenResponse,
    /// Token secret, will not be shown ever again
    secret: String,
}
