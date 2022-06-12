use crate::prelude::{ContextExtensions, HttpRequestExtensions};
use crate::privileges::repo_visibility::RepoVisibility;
use crate::user::WebUser;
use crate::{err, render_template};

use std::fmt::{Display, Formatter, Result as FmtResult};

use actix_web::{HttpRequest, Responder, web};
use anyhow::Result;
use derive_more::Display;
use gitarena_macros::route;
use qstring::QString;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::{Executor, PgPool, Postgres};
use tera::Context;

#[route("/explore", method = "GET", err = "htmx+html")]
pub(crate) async fn explore(web_user: WebUser, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let query_string = request.q_string();

    let sorting = query_string.get("sort").unwrap_or("stars_desc");
    let (sort_method, order) = Order::parse(sorting).ok_or_else(|| err!(BAD_REQUEST, "Invalid order"))?;
    let htmx_request = request.is_htmx();
    let options = ExploreOptions::parse(&query_string, &web_user, sort_method, order, htmx_request);

    let mut transaction = db_pool.begin().await?;
    let mut context = Context::new();

    context.insert_web_user(&web_user)?;

    context.try_insert("repositories", &get_repositories(&options, &mut transaction).await?)?;
    context.try_insert("options", &options)?;
    context.try_insert("query_string", query_string_without_offset(&query_string).as_str())?;

    // Only send a partial result (only the component) if it's a request by htmx
    if options.htmx_request {
        return render_template!("explore_list_component.html", context, transaction);
    }

    render_template!("explore.html", context, transaction)
}

async fn get_repositories<'e, E: Executor<'e, Database = Postgres>>(options: &ExploreOptions<'_>, executor: E) -> Result<Vec<ExploreRepo>> {
    let query = format!("select repositories.id, \
        repositories.name, \
        repositories.description, \
        repositories.owner as owner_id, \
        users.username as owner_name, \
        repositories.visibility, \
        repositories.archived, \
        repositories.disabled, \
        count(distinct stars.stargazer) as stars, \
        count(distinct issues.id) filter (where not(issues.closed = true or issues.confidential = true)) as issues \
        from repositories \
        left join stars on repositories.id = stars.repo \
        left join users on repositories.owner = users.id \
        left join issues on repositories.id = issues.repo \
     {}", options);

    Ok(sqlx::query_as::<_, ExploreRepo>(query.as_str())
        .fetch_all(executor)
        .await?)
}

#[derive(FromRow, Serialize, Deserialize, Debug)]
struct ExploreRepo {
    id: i32,
    name: String,
    description: String,
    owner_id: i32,
    owner_name: String,
    visibility: RepoVisibility,
    archived: bool,
    disabled: bool,
    stars: i64,
    issues: i64,
    #[sqlx(default)]
    merge_requests: i64,
}

#[derive(Debug, Serialize)]
struct ExploreOptions<'a> {
    archived: bool,
    forked: bool,
    mirrored: bool,
    internal: bool,
    disabled: bool,
    sort: &'a str,
    order: Order,
    offset: u32,
    htmx_request: bool
}

impl ExploreOptions<'_> {
    fn parse<'a>(query_string: &'a QString, web_user: &WebUser, sort: &'a str, order: Order, htmx_request: bool) -> ExploreOptions<'a> {
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
            htmx_request
        }
    }
}

impl Display for ExploreOptions<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("where ")?;

        if !self.archived {
            f.write_str("repositories.archived is false and ")?;
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
        f.write_str("repositories.visibility != 'private' group by repositories.id, users.id order by ")?;

        match self.sort {
            "stars" => write!(f, "stars {}, id ", self.order)?,
            "name" => write!(f, "lower(name) {}, id ", self.order)?,
            _ => write!(f, "id {} ", self.order)? // Default is repository id (creation date)
        }

        write!(f, "offset {} limit 20", self.offset)
    }
}

#[derive(Display, Debug, Serialize)]
enum Order {
    #[display(fmt = "asc")]
    #[serde(rename(serialize = "asc"))]
    Ascending,
    #[display(fmt = "desc")]
    #[serde(rename(serialize = "desc"))]
    Descending
}

impl Order {
    fn parse(input: &str) -> Option<(&str, Order)> {
        let (method, order_str) = input.split_once('_')?;
        let order = match order_str {
            "asc" => Order::Ascending,
            "desc" => Order::Descending,
            _ => return None
        };

        Some((method, order))
    }
}

fn query_string_without_offset(input: &QString) -> String {
    input.to_pairs()
        .iter()
        .filter(|(key, _)| key != &"offset")
        .map(|(key, value)| format!("{}={}", key, value))
        .collect::<Vec<String>>()
        .join("&")
}
