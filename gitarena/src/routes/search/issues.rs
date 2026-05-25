use crate::database::{Database, Pool};
use crate::issue::{IssueCache, IssueStatus};
use crate::meili::{ISSUES_MEILI_INDEX, MeiliClient};
use crate::prelude::HttpRequestExtensions;
use crate::user::WebUser;
use crate::{die, err};

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use chrono::{DateTime, Utc};
use gitarena_macros::route;
use meilisearch_sdk::search::SearchQuery;
use serde::Serialize;
use sqlx::Transaction;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/search/issues",
    params(
        ("query" = String, Query, description = "Search query string"),
        ("offset" = Option<u32>, Query, description = "Pagination offset"),
        ("limit" = Option<u32>, Query, description = "Maximum number of results (max 100, default 20)"),
    ),
    responses(
        (status = 200, description = "Issue search results", body = IssueSearchResponse),
        (status = 400, description = "Missing or invalid search query"),
        (status = 501, description = "Search disabled"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "search"
)]
#[route("/api/search/issues", method = "GET", err = "json")]
pub(crate) async fn get_issue_search(
    web_user: WebUser,
    request: HttpRequest,
    meili_client: web::Data<MeiliClient>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let Some(client) = meili_client.as_ref() else {
        die!(NOT_IMPLEMENTED, "Issue search is disabled on this instance.");
    };

    let query_string = request.q_string();

    let query = query_string
        .get("query")
        .filter(|q| !q.is_empty())
        .ok_or_else(|| err!(BAD_REQUEST, "Missing search query"))?;

    let offset: usize = query_string.get("offset").and_then(|v| v.parse().ok()).unwrap_or(0);
    let limit: usize = query_string.get("limit").and_then(|v| v.parse().ok()).unwrap_or(20).min(100);

    let mut tx = db_pool.begin().await?;

    let filter = build_issue_filter(&web_user, &mut tx).await?;

    let index = client.index(ISSUES_MEILI_INDEX);

    let mut search = SearchQuery::new(&index);
    search.with_query(query);
    search.with_offset(offset);
    search.with_limit(limit);
    search.with_filter(filter.as_str());

    let result = search.execute::<IssueCache>().await?;

    let total = result.estimated_total_hits.unwrap_or(0) as u64;
    let issues: Vec<IssueCache> = result.hits.into_iter().map(|h| h.result).collect();

    if issues.is_empty() {
        return Ok(HttpResponse::Ok().json(IssueSearchResponse { total, issues: vec![] }));
    }

    let author_ids: Vec<Uuid> = issues.iter().map(|i| i.author_id).collect();
    let repo_ids: Vec<Uuid> = issues.iter().map(|i| i.repo_id).collect();
    let issue_ids: Vec<Uuid> = issues.iter().map(|i| i.id).collect();

    let usernames = fetch_usernames(&author_ids, &mut tx).await?;
    let repo_info = fetch_repo_info(&repo_ids, &mut tx).await?;
    let comment_counts = fetch_comment_counts(&issue_ids, &mut tx).await?;

    tx.commit().await?;

    let results = issues
        .into_iter()
        .map(|issue| {
            let author_username = usernames
                .iter()
                .find(|(id, _)| *id == issue.author_id)
                .map(|(_, username)| username.clone())
                .unwrap_or_default();

            let (repo_owner, repo_name) = repo_info
                .iter()
                .find(|(id, _, _)| *id == issue.repo_id)
                .map(|(_, owner, name)| (owner.clone(), name.clone()))
                .unwrap_or_default();

            IssueSearchResult {
                index: issue.index,
                title: issue.title,
                status: issue.status,
                labels: issue.labels,
                comment_count: comment_counts
                    .iter()
                    .find(|(id, _)| *id == issue.id)
                    .map(|(_, count)| *count as i32)
                    .unwrap_or(0),
                author_username,
                repo_owner,
                repo_name,
                updated_at: issue.updated_at,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(IssueSearchResponse { total, issues: results }))
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct IssueSearchResult {
    index: i32,
    title: String,
    status: IssueStatus,
    labels: Vec<String>,
    comment_count: i32,
    author_username: String,
    repo_owner: String,
    repo_name: String,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct IssueSearchResponse {
    total: u64,
    issues: Vec<IssueSearchResult>,
}

async fn build_issue_filter(web_user: &WebUser, tx: &mut Transaction<'_, Database>) -> Result<String> {
    if let Some(user) = web_user.as_ref() {
        if user.admin {
            return Ok("confidential = false".to_owned());
        }

        let accessible_ids: Vec<Uuid> = sqlx::query_scalar(
            "select id from repositories where disabled = false and (\
                visibility in ('public', 'internal') \
                or owner_user = $1 \
                or exists (select 1 from organization_members where org_id = owner_org and user_id = $1) \
                or exists (select 1 from privileges where repo_id = id and user_id = $1) \
            )",
        )
        .bind(user.id)
        .fetch_all(&mut **tx)
        .await?;

        if accessible_ids.is_empty() {
            return Ok("confidential = false AND repo_id = \"00000000-0000-0000-0000-000000000000\"".to_owned());
        }

        let id_list = accessible_ids.iter().map(|id| format!("\"{id}\"")).collect::<Vec<_>>().join(", ");
        Ok(format!("confidential = false AND repo_id IN [{id_list}]"))
    } else {
        let public_ids: Vec<Uuid> = sqlx::query_scalar("select id from repositories where visibility = 'public' and disabled = false")
            .fetch_all(&mut **tx)
            .await?;

        if public_ids.is_empty() {
            return Ok("confidential = false AND repo_id = \"00000000-0000-0000-0000-000000000000\"".to_owned());
        }

        let id_list = public_ids.iter().map(|id| format!("\"{id}\"")).collect::<Vec<_>>().join(", ");
        Ok(format!("confidential = false AND repo_id IN [{id_list}]"))
    }
}

async fn fetch_comment_counts(ids: &[Uuid], tx: &mut Transaction<'_, Database>) -> Result<Vec<(Uuid, i64)>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    Ok(
        sqlx::query_as("select issue_id, count(*) from issue_comment_cache where issue_id = any($1) group by issue_id")
            .bind(ids)
            .fetch_all(&mut **tx)
            .await?,
    )
}

async fn fetch_usernames(ids: &[Uuid], tx: &mut Transaction<'_, Database>) -> Result<Vec<(Uuid, String)>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    Ok(sqlx::query_as("select id, username from users where id = any($1)")
        .bind(ids)
        .fetch_all(&mut **tx)
        .await?)
}

async fn fetch_repo_info(ids: &[Uuid], tx: &mut Transaction<'_, Database>) -> Result<Vec<(Uuid, String, String)>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    Ok(sqlx::query_as(
        "select r.id, coalesce(u.username, o.name, '') as owner, r.name \
         from repositories r \
         left join users u on r.owner_user = u.id \
         left join organizations o on r.owner_org = o.id \
         where r.id = any($1)",
    )
    .bind(ids)
    .fetch_all(&mut **tx)
    .await?)
}
