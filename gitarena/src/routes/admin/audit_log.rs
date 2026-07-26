use crate::database::Pool;
use crate::die;
use crate::events::EventClass;
use crate::routes::events::EventResponse;
use crate::user::WebUser;

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde::Deserialize;

#[derive(Deserialize)]
struct AdminAuditLogParams {
    #[serde(default)]
    offset: i64,
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default, rename = "type")]
    type_filter: Option<String>,
    class: Option<String>,
}

fn default_limit() -> i64 {
    20
}

#[utoipa::path(
    get,
    path = "/api/admin/audit-log",
    params(
        ("offset" = Option<i64>, Query, description = "Pagination offset"),
        ("limit" = Option<i64>, Query, description = "Maximum number of results (max 100, default 20)"),
        ("type" = Option<String>, Query, description = "Filter by event type"),
        ("class" = Option<String>, Query, description = "Filter by class: security, activity, system"),
    ),
    responses(
        (status = 200, description = "Global audit log", body = Vec<EventResponse>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin required"),
    ),
    security(("cookieAuth" = [])),
    tag = "admin"
)]
#[route("/api/admin/audit-log", method = "GET", err = "json")]
pub(crate) async fn get_admin_audit_log(web_user: WebUser, query: web::Query<AdminAuditLogParams>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if !user.admin {
        die!(FORBIDDEN, "Admin endpoints can only be called by admins");
    }

    let params = query.into_inner();
    let offset = params.offset.max(0);
    let limit = params.limit.clamp(1, 100);

    let class_filter: Option<EventClass> = match params.class.as_deref() {
        None => None,
        Some("security") => Some(EventClass::Security),
        Some("activity") => Some(EventClass::Activity),
        Some("system") => Some(EventClass::System),
        Some(_) => die!(BAD_REQUEST, "class must be one of: security, activity, system"),
    };

    let mut tx = db_pool.begin().await?;

    let base_query = "select e.id, e.trace_id, e.actor_id, e.ip_address, e.user_agent, \
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
                      where true";

    let events = match (&class_filter, &params.type_filter) {
        (Some(class), Some(type_filter)) => {
            sqlx::query_as::<_, EventResponse>(sqlx::AssertSqlSafe(format!(
                "{base_query} and e.class = $1 and e.type = $2 order by e.id desc limit $3 offset $4"
            )))
            .bind(class)
            .bind(type_filter)
            .bind(limit)
            .bind(offset)
            .fetch_all(&mut *tx)
            .await?
        }
        (Some(class), None) => {
            sqlx::query_as::<_, EventResponse>(sqlx::AssertSqlSafe(format!(
                "{base_query} and e.class = $1 order by e.id desc limit $2 offset $3"
            )))
            .bind(class)
            .bind(limit)
            .bind(offset)
            .fetch_all(&mut *tx)
            .await?
        }
        (None, Some(type_filter)) => {
            sqlx::query_as::<_, EventResponse>(sqlx::AssertSqlSafe(format!(
                "{base_query} and e.type = $1 order by e.id desc limit $2 offset $3"
            )))
            .bind(type_filter)
            .bind(limit)
            .bind(offset)
            .fetch_all(&mut *tx)
            .await?
        }
        (None, None) => {
            sqlx::query_as::<_, EventResponse>(sqlx::AssertSqlSafe(format!("{base_query} order by e.id desc limit $1 offset $2")))
                .bind(limit)
                .bind(offset)
                .fetch_all(&mut *tx)
                .await?
        }
    };

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(events))
}
