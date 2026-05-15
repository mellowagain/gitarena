use crate::database::Pool;
use crate::die;
use crate::err;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::user::{User, WebUser};

use crate::database::Database;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde::Serialize;
use serde_json::Value as JsonValue;
use sqlx::{FromRow, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserProfileResponse {
    id: Uuid,
    username: String,
    admin: bool,
    repos: Vec<UserProfileRepo>,
    stats: UserProfileStats,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserProfileStats {
    repos: i64,
    stars_earned: i64,
    stars_given: i64,
}

#[derive(FromRow, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserProfileRepo {
    id: Uuid,
    name: String,
    description: String,
    visibility: RepoVisibility,
    archived: bool,
    languages: JsonValue,
    stars: i64,
}

#[utoipa::path(
    get,
    path = "/api/users/{username}",
    params(
        ("username" = String, Path, description = "Username to look up"),
    ),
    responses(
        (status = 200, description = "User profile", body = UserProfileResponse),
        (status = 404, description = "User not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/users/{username}", method = "GET", err = "json")]
pub(crate) async fn get_user_profile(path: web::Path<String>, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let username = path.into_inner();
    let mut tx = db_pool.begin().await?;

    let target = User::find_using_name(&username, &mut tx)
        .await
        .ok_or_else(|| err!(NOT_FOUND, "User not found"))?;

    let is_self = web_user.as_ref().is_some_and(|u| u.id == target.id);
    let can_see_internal = matches!(web_user, WebUser::Authenticated(_));

    let repos = get_user_repos(target.id, is_self, can_see_internal, &mut tx).await?;

    let stats = get_user_stats(target.id, &mut tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(UserProfileResponse {
        id: target.id,
        username: target.username,
        admin: target.admin,
        repos,
        stats,
    }))
}

async fn get_user_repos(user_id: Uuid, is_self: bool, can_see_internal: bool, tx: &mut Transaction<'_, Database>) -> Result<Vec<UserProfileRepo>> {
    let mut query = "select repositories.id, \
         repositories.name, \
         repositories.description, \
         repositories.visibility, \
         repositories.archived, \
         repositories.languages, \
         count(distinct stars.stargazer) as stars \
         from repositories \
         left join stars on repositories.id = stars.repo \
         where repositories.owner_user = $1 \
         and repositories.disabled = false"
        .to_string();

    if !is_self {
        query.push_str(" and repositories.visibility != 'private'");
    }

    if !can_see_internal {
        query.push_str(" and repositories.visibility != 'internal'");
    }

    query.push_str(" group by repositories.id order by stars desc, repositories.id desc");

    Ok(sqlx::query_as(&query).bind(user_id).fetch_all(&mut **tx).await?)
}

async fn get_user_stats(user_id: Uuid, tx: &mut Transaction<'_, Database>) -> Result<UserProfileStats> {
    let row: (i64, i64, i64) = sqlx::query_as(
        "select \
         (select count(*) from repositories where owner_user = $1 and disabled = false) as repos, \
         (select count(*) from stars join repositories on stars.repo = repositories.id where repositories.owner_user = $1) as stars_earned, \
         (select count(*) from stars where stargazer = $1) as stars_given",
    )
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await?;

    Ok(UserProfileStats {
        repos: row.0,
        stars_earned: row.1,
        stars_given: row.2,
    })
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserByIdResponse {
    id: Uuid,
    username: String,
}

#[utoipa::path(
    get,
    path = "/api/users/by-id/{id}",
    params(
        ("id" = Uuid, Path, description = "User ID to look up"),
    ),
    responses(
        (status = 200, description = "User info", body = UserByIdResponse),
        (status = 404, description = "User not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/users/by-id/{id}", method = "GET", err = "json")]
pub(crate) async fn get_user_by_id(path: web::Path<Uuid>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let id = path.into_inner();
    let mut tx = db_pool.begin().await?;

    let user = sqlx::query_as::<_, (Uuid, String)>("select id, username from users where id = $1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;

    tx.commit().await?;

    match user {
        Some((id, username)) => Ok(HttpResponse::Ok().json(UserByIdResponse { id, username })),
        None => die!(NOT_FOUND, "User not found"),
    }
}
