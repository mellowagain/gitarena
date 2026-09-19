use crate::database::Pool;
use crate::die;
use crate::routes::events::default_limit;
use crate::routes::tokens::{TOKEN_SELECT, TokenResponse, can_administer, find_token};
use crate::token::TokenType;
use crate::user::WebUser;

use actix_web::web::{Data, Path, Query};
use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use gitarena_macros::route;
use serde::Deserialize;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/tokens",
    params(
        ("org" = Option<Uuid>, Query, description = "List the tokens owned by this organization"),
        ("repo" = Option<Uuid>, Query, description = "List the tokens owned by this repository"),
        ("instance" = Option<bool>, Query, description = "List the tokens that are not owned by anyone"),
        ("type" = Option<TokenType>, Query, description = "Filter by token type"),
        ("includeRevoked" = Option<bool>, Query, description = "Include revoked tokens"),
        ("offset" = Option<i64>, Query, description = "Pagination offset"),
        ("limit" = Option<i64>, Query, description = "Maximum number of results (max 100, default 20)"),
    ),
    responses(
        (status = 200, description = "List of tokens", body = Vec<TokenResponse>),
        (status = 400, description = "More than one owner specified"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
    ),
    security(("cookieAuth" = [])),
    tag = "token"
)]
#[route("/api/tokens", method = "GET", err = "json")]
pub(crate) async fn list_tokens(web_user: WebUser, query: Query<TokenListParams>, db_pool: Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let params = query.into_inner().sanitize();

    let owners = usize::from(params.org.is_some()) + usize::from(params.repo.is_some()) + usize::from(params.instance);

    if owners > 1 {
        die!(BAD_REQUEST, "Only one of org, repo or instance can be specified");
    }

    let (owner_user, owner_org, owner_repo) = if let Some(org) = params.org {
        (None, Some(org), None)
    } else if let Some(repo) = params.repo {
        (None, None, Some(repo))
    } else if params.instance {
        (None, None, None)
    } else {
        (Some(user.id), None, None)
    };

    let mut tx = db_pool.begin().await?;

    if !can_administer(owner_user, owner_org, owner_repo, &user, &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions to list these tokens");
    }

    let tokens = sqlx::query_as::<_, TokenResponse>(&format!(
        "{TOKEN_SELECT} where t.owner_user is not distinct from $1 and t.owner_org is not distinct from $2 \
         and t.owner_repo is not distinct from $3 and ($4::token_type is null or t.token_type = $4) \
         and ($5 or t.revoked_at is null) \
         group by t.id order by t.id desc limit $6 offset $7"
    ))
    .bind(owner_user)
    .bind(owner_org)
    .bind(owner_repo)
    .bind(params.type_filter)
    .bind(params.include_revoked)
    .bind(params.limit)
    .bind(params.offset)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(tokens))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenListParams {
    org: Option<Uuid>,
    repo: Option<Uuid>,
    #[serde(default)]
    instance: bool,
    #[serde(default, rename = "type")]
    type_filter: Option<TokenType>,
    #[serde(default)]
    include_revoked: bool,
    #[serde(default)]
    offset: i64,
    #[serde(default = "default_limit")]
    limit: i64,
}

impl TokenListParams {
    fn sanitize(mut self) -> Self {
        self.offset = self.offset.max(0);
        self.limit = self.limit.clamp(1, 100);
        self
    }
}

#[utoipa::path(
    get,
    path = "/api/tokens/{id}",
    params(("id" = Uuid, Path, description = "Token ID")),
    responses(
        (status = 200, description = "Token", body = TokenResponse),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Token not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "token"
)]
#[route("/api/tokens/{id}", method = "GET", err = "json")]
pub(crate) async fn get_token(path: Path<Uuid>, web_user: WebUser, db_pool: Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut tx = db_pool.begin().await?;

    let Some(token) = find_token(path.into_inner(), &mut tx).await? else {
        die!(NOT_FOUND, "Token not found");
    };

    if !can_administer(token.token.owner_user, token.token.owner_org, token.token.owner_repo, &user, &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions to view this token");
    }

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(token))
}
