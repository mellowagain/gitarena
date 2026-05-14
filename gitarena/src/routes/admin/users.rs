use crate::error::WithStatusCode;
use crate::prelude::HttpRequestExtensions;
use crate::user::{User, WebUser};
use crate::{die, err};
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use chrono::{DateTime, Local};
use fang::Serialize;
use gitarena_common::database::Pool;
use gitarena_macros::route;
use sqlx::FromRow;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/api/admin/users",
    responses(
        (status = 201, description = "Instance users", body = Vec<ExtendedUser>),
        (status = 400, description = "Invalid query strings"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin required"),
    ),
    security(("cookieAuth" = [])),
    tag = "admin"
)]
#[route("/api/admin/users", method = "GET", err = "json")]
pub(crate) async fn get_instance_users(web_user: WebUser, request: HttpRequest, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if !user.admin {
        die!(FORBIDDEN, "Admin endpoints can only be called by admins");
    }

    let query_string = request.q_string();

    let order = query_string.get("sort").map_or_else(
        || Ok::<&str, WithStatusCode>("asc"),
        |sort| match sort {
            "newest" => Ok("desc"),
            "oldest" => Ok("asc"),
            _ => die!(BAD_REQUEST, "sort needs to be either newest or oldest"),
        },
    )?;

    let limit = if let Some(limit_str) = query_string.get("limit") {
        limit_str.parse().map_err(|_| err!(BAD_REQUEST, "unable to parse limit into i32"))?
    } else {
        i32::MAX
    };

    let mut tx = db_pool.begin().await?;

    let users = sqlx::query_as::<_, ExtendedUser>(&format!(
        "select u.*, e.email, e.verified_at from users u \
         join emails e on e.owner = u.id and e.\"primary\" = true \
         order by u.id {order} \
         limit {limit}"
    ))
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(users))
}

#[derive(FromRow, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtendedUser {
    #[sqlx(flatten)]
    #[serde(flatten)]
    user: User,
    email: String,
    verified_at: Option<DateTime<Local>>,
}
