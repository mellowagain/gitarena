use crate::database::Database;
use crate::database::Pool;
use crate::die;
use crate::err;
use crate::prelude::{AwcExtensions, HttpRequestExtensions};
use crate::user::WebUser;

use std::time::Duration;

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use awc::Client;
use awc::http::StatusCode;
use gitarena_macros::{from_config, route};
use opentelemetry_instrumentation_actix_web::ClientExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use sqlx::Transaction;
use tracing::{error, instrument};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodeSearchResponse {
    result: JsonValue,
}

#[utoipa::path(
    get,
    path = "/api/search/code",
    params(
        ("query" = String, Query, description = "Zoekt query string"),
        ("limit" = Option<i32>, Query, description = "Maximum number of files to return"),
    ),
    responses(
        (status = 200, description = "Code search results", body = CodeSearchResponse),
        (status = 400, description = "Invalid search query"),
        (status = 501, description = "Code search disabled"),
        (status = 502, description = "Code search backend unavailable"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "search"
)]
#[route("/api/search/code", method = "GET", err = "json")]
pub(crate) async fn get_code_search(web_user: WebUser, request: HttpRequest, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let (enabled, address) = from_config!(
        "zoekt.enabled" => bool,
        "zoekt.address" => String,
    );

    if !enabled {
        die!(NOT_IMPLEMENTED, "Code search is disabled on this instance.");
    }

    let query_string = request.q_string();

    let query = query_string
        .get("query")
        .filter(|query| !query.is_empty())
        .ok_or_else(|| err!(BAD_REQUEST, "Missing search query"))?;

    let limit = 100.min(if let Some(limit_str) = query_string.get("limit") {
        limit_str.parse::<i32>().map_err(|_| err!(BAD_REQUEST, "could not parse limit into i32"))?
    } else {
        50
    });

    let mut tx = db_pool.begin().await?;

    let repo_ids = get_searchable_zoekt_repo_ids(&web_user, &mut tx).await?;

    tx.commit().await?;

    let mut response = Client::gitarena()
        .post(format!("{address}/api/search").as_str())
        .timeout(Duration::from_secs(35))
        .trace_request()
        .send_json(&json!({
            "Q": query,
            "RepoIDs": repo_ids,
            "Opts": {
                "NumContextLines": 1,
                "MaxDocDisplayCount": limit,
                "MaxWallTime": 30_000_000_000_i64, // 30s
            }
        }))
        .await
        .map_err(|err| {
            error!(?err, "failed to connect to zoekt");
            err!(BAD_GATEWAY)
        })?;

    let status = response.status();

    if status == StatusCode::BAD_REQUEST {
        match response.json::<ZoektErrorResponse>().await {
            Ok(error_response) => die!(BAD_REQUEST, "Invalid code search query: {}", error_response.error),
            Err(_) => die!(BAD_REQUEST, "Invalid code search query"),
        }
    }

    if !status.is_success() {
        error!(?status, ?query, "zoekt returned non 2xx status code");
        die!(BAD_GATEWAY);
    }

    let zoekt_response = response.json::<ZoektSearchResponse>().await.map_err(|err| {
        error!(?err, "failed to parse zoekt response");
        err!(BAD_GATEWAY)
    })?;

    Ok(HttpResponse::Ok().json(CodeSearchResponse { result: zoekt_response.result }))
}

#[instrument(err, skip(tx))]
async fn get_searchable_zoekt_repo_ids(web_user: &WebUser, tx: &mut Transaction<'_, Database>) -> Result<Vec<i32>> {
    let rows = if let Some(user) = web_user.as_ref() {
        if user.admin {
            sqlx::query_as::<_, (i32,)>("select zoekt_id from repositories").fetch_all(&mut **tx).await?
        } else {
            sqlx::query_as::<_, (i32,)>(
                "select zoekt_id \
                 from repositories \
                 where disabled = false \
                 and ( \
                    visibility in ('public', 'internal') \
                    or owner_user = $1 \
                    or exists (select 1 from organization_members where organization_members.org_id = repositories.owner_org and organization_members.user_id = $1) \
                    or exists (select 1 from privileges where privileges.repo_id = repositories.id and privileges.user_id = $1) \
                 )",
            )
            .bind(user.id)
            .fetch_all(&mut **tx)
            .await?
        }
    } else {
        sqlx::query_as::<_, (i32,)>("select zoekt_id from repositories where visibility = 'public' and disabled = false")
            .fetch_all(&mut **tx)
            .await?
    };

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

#[derive(Deserialize)]
struct ZoektSearchResponse {
    #[serde(rename = "Result")]
    result: JsonValue,
}

#[derive(Deserialize)]
struct ZoektErrorResponse {
    #[serde(rename = "Error")]
    error: String,
}
