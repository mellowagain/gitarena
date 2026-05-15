use crate::die;
use crate::organization::Organization;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::user::WebUser;

use crate::database::Pool;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde::Serialize;
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(FromRow, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OrgRepo {
    id: uuid::Uuid,
    name: String,
    description: String,
    visibility: RepoVisibility,
    archived: bool,
    languages: JsonValue,
    stars: i64,
}

#[utoipa::path(
    get,
    path = "/api/orgs/{name}/repos",
    params(
        ("name" = String, Path, description = "Organization name")
    ),
    responses(
        (status = 200, description = "List of repositories", body = Vec<OrgRepo>),
        (status = 404, description = "Organization not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "organization"
)]
#[route("/api/orgs/{name}/repos", method = "GET", err = "json")]
pub(crate) async fn list_repos(name: web::Path<String>, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut tx = db_pool.begin().await?;

    let Some(org) = Organization::find_by_name(&name, &mut tx).await else {
        die!(NOT_FOUND, "Organization not found");
    };

    let can_see_internal = matches!(web_user, WebUser::Authenticated(_));

    let mut query = "select repositories.id, \
         repositories.name, \
         repositories.description, \
         repositories.visibility, \
         repositories.archived, \
         repositories.languages, \
         count(distinct stars.stargazer) as stars \
         from repositories \
         left join stars on repositories.id = stars.repo \
         where repositories.owner_org = $1 \
         and repositories.disabled = false \
         and repositories.visibility != 'private'"
        .to_string();

    if !can_see_internal {
        query.push_str(" and repositories.visibility != 'internal'");
    }

    query.push_str(" group by repositories.id order by stars desc, repositories.id desc");

    let repos = sqlx::query_as::<_, OrgRepo>(&query).bind(org.id).fetch_all(&mut *tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(repos))
}
