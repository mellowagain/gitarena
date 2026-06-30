use crate::database::Pool;
use crate::routes::events::{EventListParams, EventResponse};
use crate::user::WebUser;

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;

#[utoipa::path(
    get,
    path = "/api/users/me/events",
    params(
        ("offset" = Option<i64>, Query, description = "Pagination offset"),
        ("limit" = Option<i64>, Query, description = "Maximum number of results (max 100, default 20)"),
        ("type" = Option<String>, Query, description = "Filter by event type"),
    ),
    responses(
        (status = 200, description = "Dashboard activity feed", body = Vec<EventResponse>),
        (status = 401, description = "Authentication required"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/users/me/events", method = "GET", err = "json")]
pub(crate) async fn get_dashboard_feed(web_user: WebUser, query: web::Query<EventListParams>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let params = query.into_inner().sanitize();

    let mut tx = db_pool.begin().await?;

    let type_clause = params.type_filter.as_deref().map(|_| "and e.type = $2").unwrap_or("");

    let query = format!(
        "select distinct on (e.id) e.id, null::uuid as trace_id, e.actor_id, null::inet as ip_address, null::text as user_agent, \
         e.subject_id_user, e.subject_id_org, e.subject_id_repo, \
         e.class, e.type, e.payload, \
         u.username as actor_username, \
         coalesce(su.username, so.name, sr.name) as subject_name, \
         coalesce(ruo.username, roo.name) as subject_namespace \
         from events e \
         left join users u on u.id = e.actor_id \
         left join users su on su.id = e.subject_id_user \
         left join organizations so on so.id = e.subject_id_org \
         left join repositories sr on sr.id = e.subject_id_repo \
         left join users ruo on ruo.id = sr.owner_user \
         left join organizations roo on roo.id = sr.owner_org \
         where e.class = 'activity' \
         and (\
           e.actor_id = $1 \
           or e.subject_id_repo in (\
             select r.id from repositories r \
             join organization_members om on om.org_id = r.owner_org \
             where om.user_id = $1\
           ) \
           or e.subject_id_repo in (\
             select repo from stars where stargazer = $1\
           )\
         ) \
         {type_clause} \
         order by e.id desc limit {limit} offset {offset}",
        limit = params.limit,
        offset = params.offset,
    );

    let events: Vec<EventResponse> = if let Some(ref type_filter) = params.type_filter {
        sqlx::query_as(sqlx::AssertSqlSafe(query.as_str()))
            .bind(user.id)
            .bind(type_filter)
            .fetch_all(&mut *tx)
            .await?
    } else {
        sqlx::query_as(sqlx::AssertSqlSafe(query.as_str())).bind(user.id).fetch_all(&mut *tx).await?
    };

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(events))
}
