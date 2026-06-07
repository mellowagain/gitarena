use crate::database::Pool;
use crate::die;
use crate::privileges::privilege;
use crate::privileges::repo_access::AccessLevel;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::user::{User, WebUser};

use crate::events::Event;
use actix_web::web::{Data, Json, Path};
use actix_web::{HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct CollaboratorResponse {
    user_id: Uuid,
    username: String,
    access_level: AccessLevel,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpsertCollaboratorRequest {
    username: String,
    access_level: AccessLevel,
}

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/collaborators",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "List of collaborators", body = Vec<CollaboratorResponse>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/collaborators", method = "GET", err = "json")]
pub(crate) async fn list_collaborators(repo: Repository, web_user: WebUser, db_pool: Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let mut tx = db_pool.begin().await?;

    if !privilege::check_admin(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let rows: Vec<(Uuid, String, AccessLevel)> = sqlx::query_as(
        "select p.user_id, u.username, p.access_level \
         from privileges p \
         join users u on u.id = p.user_id \
         where p.repo_id = $1",
    )
    .bind(repo.id)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(
        rows.into_iter()
            .map(|(user_id, username, access_level)| CollaboratorResponse {
                user_id,
                username,
                access_level,
            })
            .collect::<Vec<_>>(),
    ))
}

#[utoipa::path(
    put,
    path = "/api/repos/{namespace}/{repository}/collaborators",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    request_body = UpsertCollaboratorRequest,
    responses(
        (status = 200, description = "Collaborator added or updated", body = CollaboratorResponse),
        (status = 400, description = "Invalid request or cannot modify repo owner"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Repository or target user not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/collaborators", method = "PUT", err = "json")]
pub(crate) async fn upsert_collaborator(
    repo: Repository,
    web_user: WebUser,
    body: Json<UpsertCollaboratorRequest>,
    request: HttpRequest,
    db_pool: Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    if !privilege::check_admin(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let Some(target) = User::find_using_name(&body.username, &mut tx).await else {
        die!(NOT_FOUND, "User not found");
    };

    if repo.owner_user == Some(target.id) {
        die!(BAD_REQUEST, "Cannot modify privileges of the repository owner");
    }

    if body.access_level == AccessLevel::Viewer && repo.visibility != RepoVisibility::Private {
        die!(BAD_REQUEST, "Viewer collaborators can only be added to private repositories");
    }

    let old_privilege = privilege::get_repo_privilege(&repo, &user, &mut tx).await?;

    sqlx::query(
        "insert into privileges (user_id, repo_id, access_level) values ($1, $2, $3) \
         on conflict (user_id, repo_id) do update set access_level = excluded.access_level",
    )
    .bind(target.id)
    .bind(repo.id)
    .bind(&body.access_level)
    .execute(&mut *tx)
    .await?;

    Event::new(
        if old_privilege.is_some() { "privilege.changed" } else { "privilege.granted" },
        user.id,
        &request,
        (&target).into(),
        Some(json!({
            "repo": repo.id,
            "old_level": old_privilege.map(|level| level.access_level),
            "new_level": body.access_level
        })),
    )
    .save(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(CollaboratorResponse {
        user_id: target.id,
        username: target.username,
        access_level: body.into_inner().access_level,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/repos/{namespace}/{repository}/collaborators/{username}",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("username" = String, Path, description = "Username of the collaborator to remove"),
    ),
    responses(
        (status = 204, description = "Collaborator removed"),
        (status = 400, description = "Cannot remove the repository owner"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Repository or collaborator not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/collaborators/{username}", method = "DELETE", err = "json")]
pub(crate) async fn remove_collaborator(
    repo: Repository,
    path: Path<(String, String, String)>,
    web_user: WebUser,
    request: HttpRequest,
    db_pool: Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    if !privilege::check_admin(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let (_, _, target_username) = path.into_inner();

    let Some(target) = User::find_using_name(&target_username, &mut tx).await else {
        die!(NOT_FOUND, "User not found");
    };

    if repo.owner_user == Some(target.id) {
        die!(BAD_REQUEST, "Cannot remove the repository owner");
    }

    let result = sqlx::query("delete from privileges where user_id = $1 and repo_id = $2")
        .bind(target.id)
        .bind(repo.id)
        .execute(&mut *tx)
        .await?;

    if result.rows_affected() == 0 {
        die!(NOT_FOUND, "Collaborator not found");
    }

    Event::new(
        "privilege.revoked",
        user.id,
        &request,
        (&target).into(),
        Some(json!({
            "repo": repo.id
        })),
    )
    .save(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}
