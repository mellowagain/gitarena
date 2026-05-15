use crate::config::get_setting;
use crate::database::Pool;
use crate::die;
use crate::user::WebUser;
use actix_web::{HttpResponse, Responder, web};
use anyhow::{Context, Result};
use fang::Serialize;
use gitarena_macros::route;
use std::path::Path;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/api/admin/stats",
    responses(
        (status = 201, description = "Instance stats", body = InstanceStats),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin required"),
    ),
    security(("cookieAuth" = [])),
    tag = "admin"
)]
#[route("/api/admin/stats", method = "GET", err = "json")]
pub(crate) async fn get_instance_stats(web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if !user.admin {
        die!(FORBIDDEN, "Admin endpoints can only be called by admins");
    }

    let mut tx = db_pool.begin().await?;

    let (users, orgs, repositories): (i64, i64, i64) = sqlx::query_as(
        "select \
            (select count(*) from users) as user_count, \
            (select count(*) from organizations) as orgs_count, \
            (select count(*) from repositories) as repo_count",
    )
    .fetch_one(&mut *tx)
    .await?;

    let repos_dir: String = get_setting("repositories.base_dir", &mut tx).await?;

    tx.commit().await?;

    let path = Path::new(&repos_dir);
    let stats = fs2::statvfs(path).context("failed to get statvfs for the repos directory")?;

    let total_space = stats.total_space();

    Ok(HttpResponse::Ok().json(InstanceStats {
        users,
        orgs,
        repositories,
        total_space,
        used_space: total_space - stats.available_space(),
    }))
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstanceStats {
    /// User amount
    users: i64,
    /// Organization amount
    orgs: i64,
    /// Repository amount
    repositories: i64,
    /// Total disk space in bytes for repositories directory
    total_space: u64,
    /// Used disk space in bytes for repositories directory
    used_space: u64,
}
