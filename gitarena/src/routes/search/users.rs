use crate::meili::{MeiliClient, USERS_MEILI_INDEX};
use crate::prelude::HttpRequestExtensions;
use crate::user::WebUser;
use crate::{die, err};

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use meilisearch_sdk::search::SearchQuery;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserSearchResponse {
    total: u64,
    users: Vec<SearchUser>,
}

#[derive(Deserialize, Serialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchUser {
    id: Uuid,
    username: String,
    admin: bool,
}

#[utoipa::path(
    get,
    path = "/api/search/users",
    params(
        ("query" = String, Query, description = "Search query string"),
        ("offset" = Option<u32>, Query, description = "Pagination offset"),
        ("limit" = Option<u32>, Query, description = "Maximum number of results (max 100, default 20)"),
    ),
    responses(
        (status = 200, description = "User search results", body = UserSearchResponse),
        (status = 400, description = "Missing or invalid search query"),
        (status = 501, description = "Search disabled"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "search"
)]
#[route("/api/search/users", method = "GET", err = "json")]
pub(crate) async fn get_user_search(web_user: WebUser, request: HttpRequest, meili_client: web::Data<MeiliClient>) -> Result<impl Responder> {
    let Some(client) = meili_client.as_ref() else {
        die!(NOT_IMPLEMENTED, "Repository search is disabled on this instance.");
    };

    let query_string = request.q_string();

    let query = query_string
        .get("query")
        .filter(|q| !q.is_empty())
        .ok_or_else(|| err!(BAD_REQUEST, "Missing search query"))?;

    let offset: usize = query_string.get("offset").and_then(|v| v.parse().ok()).unwrap_or(0);
    let limit: usize = query_string.get("limit").and_then(|v| v.parse().ok()).unwrap_or(20).min(100);

    let is_admin = web_user.as_ref().is_some_and(|u| u.admin);

    let index = client.index(USERS_MEILI_INDEX);

    let mut search = SearchQuery::new(&index);
    search.with_query(query);
    search.with_offset(offset);
    search.with_limit(limit);

    if !is_admin {
        search.with_filter("disabled = false");
    }

    let result = search.execute::<SearchUser>().await?;

    let total = result.estimated_total_hits.unwrap_or(0) as u64;
    let users: Vec<SearchUser> = result.hits.into_iter().map(|h| h.result).collect();

    Ok(HttpResponse::Ok().json(UserSearchResponse { total, users }))
}
