use crate::database::Pool;
use crate::err;
use crate::user::{User, WebUser};

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::{NoContext, Timestamp, Uuid};

const CONTRIBUTION_TYPES: &[&str] = &["issue.opened", "issue.closed", "pr.opened", "pr.merged", "pr.reviewed"];

#[utoipa::path(
    get,
    path = "/api/users/{username}/contributions",
    params(
        ("username" = String, Path, description = "Username to look up"),
        ("year" = Option<i32>, Query, description = "Year to query (defaults to last 365 days)"),
    ),
    responses(
        (status = 200, description = "Contribution graph data", body = ContributionsResponse),
        (status = 404, description = "User not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/users/{username}/contributions", method = "GET", err = "json")]
pub(crate) async fn get_contributions(
    path: web::Path<String>,
    web_user: WebUser,
    query: web::Query<ContributionParams>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let username = path.into_inner();
    let mut tx = db_pool.begin().await?;

    let target = User::find_using_name(&username, &mut tx)
        .await
        .ok_or_else(|| err!(NOT_FOUND, "User not found"))?;

    let year = query.into_inner().year;

    let viewer_id = web_user.as_ref().map(|u| u.id);
    let is_self = viewer_id.is_some_and(|id| id == target.id);

    let (start, end) = date_range(year);
    let start_ms = u64::try_from(start.timestamp_millis()).unwrap_or(0);
    let end_ms = u64::try_from(end.timestamp_millis()).unwrap_or(0);

    let lower = uuid_lower_bound(start_ms);
    let upper = uuid_upper_bound(end_ms);

    let event_privacy = if is_self {
        String::new()
    } else if let Some(vid) = viewer_id {
        format!(
            "and (r.visibility is null or r.visibility != 'private' \
             or r.owner_user = '{vid}' \
             or exists (select 1 from organization_members om join repositories ro on ro.owner_org = om.org_id where om.user_id = '{vid}' and ro.id = e.subject_id_repo) \
             or exists (select 1 from privileges p where p.repo_id = e.subject_id_repo and p.user_id = '{vid}'))"
        )
    } else {
        "and (r.visibility is null or r.visibility != 'private')".to_owned()
    };

    let commit_privacy = if is_self {
        String::new()
    } else if let Some(viewer_id) = viewer_id {
        format!(
            "and (r.visibility is null or r.visibility != 'private' \
             or r.owner_user = '{viewer_id}' \
             or exists (select 1 from organization_members om join repositories ro on ro.owner_org = om.org_id where om.user_id = '{viewer_id}' and ro.id = cc.repo_id) \
             or exists (select 1 from privileges p where p.repo_id = cc.repo_id and p.user_id = '{viewer_id}'))"
        )
    } else {
        "and (r.visibility is null or r.visibility != 'private')".to_owned()
    };

    let type_list = CONTRIBUTION_TYPES.iter().map(|t| format!("'{t}'")).collect::<Vec<_>>().join(", ");

    let event_query = format!(
        "select e.id \
         from events e \
         left join repositories r on r.id = e.subject_id_repo \
         where e.actor_id = $1 and e.id >= $2 and e.id < $3 \
         and e.type in ({type_list}) \
         {event_privacy}"
    );

    let contributor_query = format!(
        "select cc.author_date, count(*)::int as count \
         from commit_contributions cc \
         join repositories r on r.id = cc.repo_id \
         where cc.user_id = $1 \
         and cc.author_date >= $2 and cc.author_date < $3 \
         {commit_privacy} \
         group by cc.author_date"
    );

    let event_rows = sqlx::query_as::<_, EventContribRow>(&event_query)
        .bind(target.id)
        .bind(lower)
        .bind(upper)
        .fetch_all(&mut *tx)
        .await?;

    let contributor_rows = sqlx::query_as::<_, CommitContribRow>(&contributor_query)
        .bind(target.id)
        .bind(start.date_naive())
        .bind(end.date_naive())
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    let mut daily: HashMap<String, i32> = HashMap::new();

    for row in contributor_rows {
        let date_str = row.author_date.format("%Y-%m-%d").to_string();
        *daily.entry(date_str).or_insert(0) += row.count;
    }

    for row in event_rows {
        let timestamp_ms = row.id.get_timestamp().map_or(0, |ts| {
            let (secs, nanos) = ts.to_unix();
            secs * 1000 + u64::from(nanos) / 1_000_000
        });

        let dt = Utc
            .timestamp_millis_opt(i64::try_from(timestamp_ms).unwrap_or(i64::MAX))
            .single()
            .unwrap_or_default();

        let date_str = dt.format("%Y-%m-%d").to_string();

        *daily.entry(date_str).or_insert(0) += 1;
    }

    let mut contributions: Vec<ContributionDay> = Vec::new();
    let mut cursor: NaiveDate = start.date_naive();
    let end_date: NaiveDate = end.date_naive();

    while cursor < end_date {
        let date_str = cursor.format("%Y-%m-%d").to_string();
        let count = *daily.get(&date_str).unwrap_or(&0);
        contributions.push(ContributionDay { date: date_str, count });
        cursor += Duration::days(1);
    }

    Ok(HttpResponse::Ok().json(ContributionsResponse { contributions }))
}

#[derive(Deserialize)]
struct ContributionParams {
    year: Option<i32>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContributionsResponse {
    contributions: Vec<ContributionDay>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContributionDay {
    date: String,
    count: i32,
}

#[derive(FromRow)]
struct EventContribRow {
    id: Uuid,
}

#[derive(FromRow)]
struct CommitContribRow {
    author_date: NaiveDate,
    count: i32,
}

fn uuid_lower_bound(ts_ms: u64) -> Uuid {
    let ts = Timestamp::from_unix(NoContext, ts_ms / 1000, u32::try_from(ts_ms % 1000 * 1_000_000).unwrap_or(0));
    let mut bytes = *Uuid::new_v7(ts).as_bytes();
    bytes[6] &= 0xF0; // keep version nibble (0x7_), zero counter high nibble
    bytes[7] = 0x00;
    bytes[8] &= 0xC0; // keep variant bits (0b10______), zero random → 0x80
    bytes[9..].fill(0x00);
    Uuid::from_bytes(bytes)
}

fn uuid_upper_bound(ts_ms: u64) -> Uuid {
    let ts = Timestamp::from_unix(NoContext, ts_ms / 1000, u32::try_from(ts_ms % 1000 * 1_000_000).unwrap_or(0));
    let mut bytes = *Uuid::new_v7(ts).as_bytes();
    bytes[6] |= 0x0F; // keep version nibble (0x7_), set counter high nibble to 0xF → 0x7F
    bytes[7] = 0xFF;
    bytes[8] |= 0x3F; // keep variant bits (0b10______), max random → 0xBF
    bytes[9..].fill(0xFF);
    Uuid::from_bytes(bytes)
}

fn date_range(year: Option<i32>) -> (DateTime<Utc>, DateTime<Utc>) {
    let now = Utc::now();

    if let Some(y) = year {
        let start = NaiveDate::from_ymd_opt(y, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end = NaiveDate::from_ymd_opt(y + 1, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc();
        (start, end)
    } else {
        let today = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let start = today - Duration::days(365);
        let end = today + Duration::days(1);
        (start, end)
    }
}
