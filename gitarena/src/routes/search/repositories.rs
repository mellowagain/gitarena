use crate::database::{Database, Pool};
use crate::meili::{MeiliClient, REPOS_MEILI_INDEX};
use crate::prelude::HttpRequestExtensions;
use crate::repository::Repository;
use crate::routes::explore::ExploreRepo;
use crate::user::WebUser;
use crate::{die, err};

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use meilisearch_sdk::search::SearchQuery;
use serde::Serialize;
use sqlx::{FromRow, Transaction};
use tracing::instrument;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepoSearchResponse {
    total: u64,
    repositories: Vec<ExploreRepo>,
}

#[utoipa::path(
    get,
    path = "/api/search/repositories",
    params(
        ("query" = String, Query, description = "Search query string"),
        ("offset" = Option<u32>, Query, description = "Pagination offset"),
        ("limit" = Option<u32>, Query, description = "Maximum number of results (max 100, default 20)"),
    ),
    responses(
        (status = 200, description = "Repository search results", body = RepoSearchResponse),
        (status = 400, description = "Missing or invalid search query"),
        (status = 501, description = "Search disabled"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "search"
)]
#[route("/api/search/repositories", method = "GET", err = "json")]
pub(crate) async fn get_repo_search(
    web_user: WebUser,
    request: HttpRequest,
    db_pool: web::Data<Pool>,
    meili_client: web::Data<MeiliClient>,
) -> Result<impl Responder> {
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

    let mut tx = db_pool.begin().await?;
    let filter = build_repo_filter(&web_user, &mut tx).await?;
    tx.commit().await?;

    let index = client.index(REPOS_MEILI_INDEX);

    let mut search = SearchQuery::new(&index);
    search.with_query(query);
    search.with_offset(offset);
    search.with_limit(limit);

    if let Some(ref f) = filter {
        search.with_filter(f.as_str());
    }

    let result = search.execute::<Repository>().await?;

    let total = result.estimated_total_hits.unwrap_or(0) as u64;
    let repos: Vec<Repository> = result.hits.into_iter().map(|h| h.result).collect();
    let ids: Vec<Uuid> = repos.iter().map(|r| r.id).collect();

    let mut tx = db_pool.begin().await?;
    let extra = fetch_extra(&ids, &mut tx).await?;
    tx.commit().await?;

    let repositories = repos
        .into_iter()
        .map(|repo| {
            let (owner_name, stars, issues) = extra
                .iter()
                .find(|e| e.id == repo.id)
                .map(|e| (e.owner_name.clone(), e.stars, e.issues))
                .unwrap_or_else(|| (String::new(), 0, 0));

            ExploreRepo::from_repo(repo, owner_name, stars, issues)
        })
        .collect();

    Ok(HttpResponse::Ok().json(RepoSearchResponse { total, repositories }))
}

#[instrument(err, skip(tx))]
async fn build_repo_filter(web_user: &WebUser, tx: &mut Transaction<'_, Database>) -> Result<Option<String>> {
    if let Some(user) = web_user.as_ref() {
        if user.admin {
            return Ok(None);
        }

        let extra_ids: Vec<Uuid> = sqlx::query_scalar(
            "select repositories.id from repositories \
             where exists (select 1 from organization_members where organization_members.org_id = repositories.owner_org and organization_members.user_id = $1) \
             or exists (select 1 from privileges where privileges.repo_id = repositories.id and privileges.user_id = $1)",
        )
        .bind(user.id)
        .fetch_all(&mut **tx)
        .await?;

        let base = format!("disabled = false AND (visibility IN [\"public\", \"internal\"] OR owner_user = \"{}\"", user.id);

        let filter = if extra_ids.is_empty() {
            format!("{base})")
        } else {
            let id_list = extra_ids.iter().map(|id| format!("\"{id}\"")).collect::<Vec<_>>().join(", ");
            format!("{base} OR id IN [{id_list}])")
        };

        Ok(Some(filter))
    } else {
        Ok(Some("visibility = \"public\" AND disabled = false".to_owned()))
    }
}

#[derive(FromRow)]
struct RepoExtra {
    id: Uuid,
    owner_name: String,
    stars: i64,
    issues: i64,
}

async fn fetch_extra(ids: &[Uuid], tx: &mut Transaction<'_, Database>) -> Result<Vec<RepoExtra>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }

    Ok(sqlx::query_as(
        "select repositories.id, \
        coalesce(u.username, o.name) as owner_name, \
        count(distinct stars.stargazer) as stars, \
        count(distinct issues.id) filter (where not(issues.status != 'open' or issues.confidential = true)) as issues \
        from repositories \
        left join stars on repositories.id = stars.repo \
        left join users u on repositories.owner_user = u.id \
        left join organizations o on repositories.owner_org = o.id \
        left join issue_cache issues on repositories.id = issues.repo_id \
        where repositories.id = any($1) \
        group by repositories.id, u.username, o.name",
    )
    .bind(ids)
    .fetch_all(&mut **tx)
    .await?)
}
