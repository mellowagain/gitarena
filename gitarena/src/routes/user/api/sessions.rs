use crate::die;
use crate::geoip;
use crate::session::Session;
use crate::user::WebUser;
use gitarena_common::database::Pool;

use actix_identity::Identity;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use chrono::{DateTime, Local};
use gitarena_macros::route;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionResponse {
    hash: String,
    ip_address: String,
    user_agent: String,
    city: Option<String>,
    country: Option<String>,
    is_current: bool,
    updated_at: DateTime<Local>,
}

impl SessionResponse {
    pub(crate) fn new(session: Session, is_current: bool) -> Self {
        let ip = session.ip_address.ip();
        let (city, country) = geoip::lookup(ip);

        SessionResponse {
            is_current,
            hash: session.hash,
            ip_address: ip.to_string(),
            user_agent: session.user_agent,
            city,
            country,
            updated_at: session.updated_at,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/sessions",
    responses(
        (status = 200, description = "List of active sessions", body = Vec<SessionResponse>),
        (status = 401, description = "Authentication required"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/sessions", method = "GET", err = "json")]
pub(crate) async fn get_sessions(id: Identity, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut transaction = db_pool.begin().await?;

    let sessions = sqlx::query_as::<_, Session>("select * from sessions where user_id = $1 order by updated_at desc")
        .bind(user.id)
        .fetch_all(&mut *transaction)
        .await?;

    let current_session = Session::from_identity(id.identity(), &mut transaction)
        .await?
        .expect("authenticated user to have a session");

    transaction.commit().await?;

    let response: Vec<SessionResponse> = sessions
        .into_iter()
        .map(|s| {
            let is_current = s.hash == current_session.hash;
            SessionResponse::new(s, is_current)
        })
        .collect();

    Ok(HttpResponse::Ok().json(response))
}

#[utoipa::path(
    delete,
    path = "/api/sessions/{hash}",
    params(("hash" = String, Path, description = "Session hash to revoke")),
    responses(
        (status = 204, description = "Session revoked"),
        (status = 400, description = "Cannot revoke the current session"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Session not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/sessions/{hash}", method = "DELETE", err = "json")]
pub(crate) async fn delete_session(path: web::Path<String>, id: Identity, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let target_hash = path.into_inner();

    let mut transaction = db_pool.begin().await?;

    let current_session = Session::from_identity(id.identity(), &mut transaction)
        .await?
        .expect("authenticated user to have a session");

    if target_hash == current_session.hash {
        die!(BAD_REQUEST, "Use /api/auth/logout to end the current session");
    }

    let rows = sqlx::query("delete from sessions where hash = $1 and user_id = $2")
        .bind(target_hash.as_str())
        .bind(user.id)
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    if rows == 0 {
        die!(NOT_FOUND, "Session not found");
    }

    transaction.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    delete,
    path = "/api/sessions",
    responses(
        (status = 204, description = "All other sessions revoked"),
        (status = 401, description = "Authentication required"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/sessions", method = "DELETE", err = "json")]
pub(crate) async fn delete_all_sessions(id: Identity, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut transaction = db_pool.begin().await?;

    let current_session = Session::from_identity(id.identity(), &mut transaction)
        .await?
        .expect("authenticated user to have a session");

    sqlx::query("delete from sessions where user_id = $1 and hash != $2")
        .bind(user.id)
        .bind(&current_session.hash)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}
