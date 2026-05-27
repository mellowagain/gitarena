use crate::database::Pool;
use crate::err;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::user::WebUser;

use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::database::Database;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use chrono::{DateTime, Utc};
use derive_more::Display;
use gitarena_macros::route;
use qstring::QString;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub(crate) struct ExploreResponse {
    repositories: Vec<ExploreRepo>,
}

#[utoipa::path(
    get,
    path = "/api/explore",
    params(
        ("sort" = Option<String>, Query, description = "Sort order"),
        ("archived" = Option<u8>, Query, description = "Include archived repositories"),
        ("fork" = Option<u8>, Query, description = "Include forked repositories"),
        ("mirror" = Option<u8>, Query, description = "Include mirrored repositories"),
        ("offset" = Option<u32>, Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of repositories", body = ExploreResponse),
        (status = 400, description = "Invalid sort order"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "explore"
)]
#[route("/api/explore", method = "GET", err = "json")]
pub(crate) async fn explore(web_user: WebUser, request: HttpRequest, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let query_string = QString::from(request.query_string());

    let sorting = query_string.get("sort").unwrap_or("stars_desc");
    let (sort_method, order) = Order::parse(sorting).ok_or_else(|| err!(BAD_REQUEST, "Invalid order"))?;
    let options = ExploreOptions::parse(&query_string, &web_user, sort_method, order);

    let mut transaction = db_pool.begin().await?;
    let repositories = get_repositories(&options, &mut transaction).await?;
    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(ExploreResponse { repositories }))
}

async fn get_repositories(options: &ExploreOptions<'_>, tx: &mut Transaction<'_, Database>) -> Result<Vec<ExploreRepo>> {
    // Resolve owner name: check users table first, then organizations
    let query = format!(
        "select repositories.id, \
        repositories.name, \
        repositories.description, \
        coalesce(repositories.owner_user, repositories.owner_org) as owner_id, \
        coalesce(u.username, o.name) as owner_name, \
        repositories.visibility, \
        repositories.archived_at, \
        repositories.disabled, \
        repositories.languages, \
        count(distinct stars.stargazer) as stars, \
        count(distinct issues.id) filter (where not(issues.open = false or issues.confidential = true)) as issues \
        from repositories \
        left join stars on repositories.id = stars.repo \
        left join users u on repositories.owner_user = u.id \
        left join organizations o on repositories.owner_org = o.id \
        left join issue_cache issues on repositories.id = issues.repo_id \
        {options}",
    );

    Ok(sqlx::query_as::<_, ExploreRepo>(query.as_str()).fetch_all(&mut **tx).await?)
}

#[derive(FromRow, Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExploreRepo {
    id: Uuid,
    name: String,
    description: String,
    owner_id: Uuid,
    owner_name: String,
    visibility: RepoVisibility,
    archived_at: Option<DateTime<Utc>>,
    disabled: bool,
    languages: JsonValue,
    stars: i64,
    issues: i64,
    #[sqlx(default)]
    merge_requests: i64,
}

impl ExploreRepo {
    pub(crate) fn id(&self) -> Uuid {
        self.id
    }

    pub(crate) fn from_repo(repo: crate::repository::Repository, owner_name: String, stars: i64, issues: i64) -> Self {
        let owner_id = repo.owner_user.or(repo.owner_org).unwrap_or_default();
        let languages = serde_json::to_value(&repo.languages).unwrap_or_default();

        ExploreRepo {
            id: repo.id,
            name: repo.name,
            description: repo.description,
            owner_id,
            owner_name,
            visibility: repo.visibility,
            archived_at: repo.archived_at,
            disabled: repo.disabled,
            languages,
            stars,
            issues,
            merge_requests: 0,
        }
    }
}

#[derive(Debug)]
struct ExploreOptions<'a> {
    archived: bool,
    forked: bool,
    mirrored: bool,
    internal: bool,
    disabled: bool,
    sort: &'a str,
    order: Order,
    offset: u32,
}

impl ExploreOptions<'_> {
    fn parse<'a>(query_string: &'a QString, web_user: &WebUser, sort: &'a str, order: Order) -> ExploreOptions<'a> {
        let (internal, disabled) = web_user.as_ref().map_or_else(|| (false, false), |user| (true, user.admin));

        ExploreOptions {
            archived: query_string.get("archived").map_or_else(|| true, |value| value == "1"),
            forked: query_string.get("fork").map_or_else(|| true, |value| value == "1"),
            mirrored: query_string.get("mirror").map_or_else(|| true, |value| value == "1"),
            internal,
            disabled,
            sort,
            order,
            offset: query_string.get("offset").map_or_else(|| 0, |value| value.parse::<u32>().unwrap_or(0)),
        }
    }
}

impl Display for ExploreOptions<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("where ")?;

        if !self.archived {
            f.write_str("repositories.archived_at is null and ")?;
        }

        if !self.forked {
            f.write_str("repositories.forked_from is null and ")?;
        }

        if !self.mirrored {
            f.write_str("repositories.mirrored_from is null and ")?;
        }

        if !self.internal {
            f.write_str("repositories.visibility != 'internal' and ")?;
        }

        if !self.disabled {
            f.write_str("repositories.disabled is false and ")?;
        }

        // Private repositories are hidden in the public explore page
        // TODO: Display them if the logged in user has permission to view them
        f.write_str("repositories.visibility != 'private' group by repositories.id, u.username, o.name order by ")?;

        match self.sort {
            "stars" => write!(f, "stars {}, id ", self.order)?,
            "name" => write!(f, "lower(name) {}, id ", self.order)?,
            _ => write!(f, "id {} ", self.order)?, // Default is repository id (creation date)
        }

        write!(f, "offset {} limit 20", self.offset)
    }
}

#[derive(Display, Debug, Serialize)]
enum Order {
    #[display("asc")]
    #[serde(rename(serialize = "asc"))]
    Ascending,
    #[display("desc")]
    #[serde(rename(serialize = "desc"))]
    Descending,
}

impl Order {
    fn parse(input: &str) -> Option<(&str, Order)> {
        let (method, order_str) = input.split_once('_')?;
        let order = match order_str {
            "asc" => Order::Ascending,
            "desc" => Order::Descending,
            _ => return None,
        };

        Some((method, order))
    }
}
