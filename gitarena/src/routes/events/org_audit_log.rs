use crate::database::Pool;
use crate::die;
use crate::organization::{OrgMember, OrgRole, Organization};
use crate::routes::events::{EventListParams, EventResponse};
use crate::user::WebUser;

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;

#[utoipa::path(
    get,
    path = "/api/orgs/{name}/audit-log",
    params(
        ("name" = String, Path, description = "Organization name"),
        ("offset" = Option<i64>, Query, description = "Pagination offset"),
        ("limit" = Option<i64>, Query, description = "Maximum number of results (max 100, default 20)"),
        ("type" = Option<String>, Query, description = "Filter by event type"),
    ),
    responses(
        (status = 200, description = "Organization audit log", body = Vec<EventResponse>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin or owner role required"),
        (status = 404, description = "Organization not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "organization"
)]
#[route("/api/orgs/{name}/audit-log", method = "GET", err = "json")]
pub(crate) async fn get_org_audit_log(
    path: web::Path<String>,
    web_user: WebUser,
    query: web::Query<EventListParams>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let actor = web_user.into_user()?;
    let org_name = path.into_inner();

    let mut tx = db_pool.begin().await?;

    let Some(org) = Organization::find_by_name(&org_name, &mut tx).await else {
        die!(NOT_FOUND, "Organization not found");
    };

    let role = OrgMember::get_role(org.id, actor.id, &mut tx).await?;
    if !role.is_some_and(|r| OrgMember::has_permission(r, OrgRole::Admin)) {
        die!(FORBIDDEN, "Admin or owner role required to view org audit log");
    }

    let params = query.into_inner().sanitize();

    let base_query = "select e.id, null::uuid as trace_id, e.actor_id, e.ip_address, e.user_agent, \
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
                      where e.subject_id_org = $1";

    let events: Vec<EventResponse> = if let Some(ref type_filter) = params.type_filter {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "{base_query} and e.type = $2 order by e.id desc limit $3 offset $4"
        )))
        .bind(org.id)
        .bind(type_filter)
        .bind(params.limit)
        .bind(params.offset)
        .fetch_all(&mut *tx)
        .await?
    } else {
        sqlx::query_as(sqlx::AssertSqlSafe(format!("{base_query} order by e.id desc limit $2 offset $3")))
            .bind(org.id)
            .bind(params.limit)
            .bind(params.offset)
            .fetch_all(&mut *tx)
            .await?
    };

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(events))
}
