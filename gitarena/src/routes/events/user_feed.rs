use crate::database::Pool;
use crate::err;
use crate::routes::events::{EventListParams, EventResponse};
use crate::user::{User, WebUser};

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;

#[utoipa::path(
    get,
    path = "/api/users/{username}/events",
    params(
        ("username" = String, Path, description = "Username to look up"),
        ("offset" = Option<i64>, Query, description = "Pagination offset"),
        ("limit" = Option<i64>, Query, description = "Maximum number of results (max 100, default 20)"),
        ("type" = Option<String>, Query, description = "Filter by event type"),
    ),
    responses(
        (status = 200, description = "User activity feed", body = Vec<EventResponse>),
        (status = 404, description = "User not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/users/{username}/events", method = "GET", err = "json")]
pub(crate) async fn get_user_feed(
    path: web::Path<String>,
    web_user: WebUser,
    query: web::Query<EventListParams>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let username = path.into_inner();
    let mut tx = db_pool.begin().await?;

    let target = User::find_using_name(&username, &mut tx)
        .await
        .ok_or_else(|| err!(NOT_FOUND, "User not found"))?;

    let viewer_id = web_user.as_ref().map(|u| u.id);
    let params = query.into_inner().sanitize();

    let privacy_clause = if viewer_id.map(|id| id == target.id).unwrap_or(false) {
        String::new()
    } else if let Some(vid) = viewer_id {
        format!(
            "and (r.visibility is null or r.visibility = 'public' \
             or (r.visibility = 'internal') \
             or (r.visibility = 'private' and (\
               r.owner_user = '{vid}' \
               or exists (select 1 from organization_members om join repositories ro on ro.owner_org = om.org_id where om.user_id = '{vid}' and ro.id = e.subject_id_repo) \
               or exists (select 1 from privileges p where p.repo_id = e.subject_id_repo and p.user_id = '{vid}')\
             )))"
        )
    } else {
        "and (r.visibility is null or r.visibility = 'public')".to_owned()
    };

    let type_clause = params.type_filter.as_deref().map(|_| "and e.type = $4").unwrap_or("");

    let query = format!(
        "select e.id, null::uuid as trace_id, e.actor_id, null::inet as ip_address, null::text as user_agent, \
         e.subject_id_user, e.subject_id_org, e.subject_id_repo, \
         e.class, e.type, e.payload, \
         u.username as actor_username, \
         coalesce(su.username, so.name, sr.name) as subject_name, \
         coalesce(ruo.username, roo.name) as subject_namespace \
         from events e \
         left join users u on u.id = e.actor_id \
         left join repositories r on r.id = e.subject_id_repo \
         left join users su on su.id = e.subject_id_user \
         left join organizations so on so.id = e.subject_id_org \
         left join repositories sr on sr.id = e.subject_id_repo \
         left join users ruo on ruo.id = sr.owner_user \
         left join organizations roo on roo.id = sr.owner_org \
         where e.actor_id = $1 and e.class = 'activity' \
         {privacy_clause} {type_clause} \
         order by e.id desc limit $2 offset $3"
    );

    let events: Vec<EventResponse> = if let Some(ref type_filter) = params.type_filter {
        sqlx::query_as(&query)
            .bind(target.id)
            .bind(params.limit)
            .bind(params.offset)
            .bind(type_filter)
            .fetch_all(&mut *tx)
            .await?
    } else {
        sqlx::query_as(&query)
            .bind(target.id)
            .bind(params.limit)
            .bind(params.offset)
            .fetch_all(&mut *tx)
            .await?
    };

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(events))
}
