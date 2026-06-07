use crate::database::Pool;
use crate::die;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::user::WebUser;

use crate::events::Event;
use crate::meili::MeiliClient;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub(crate) struct ArchiveRequest {
    /// true archives the repo, false unarchives the repo
    pub(crate) archive: bool,
}

#[utoipa::path(
    patch,
    path = "/api/repos/{namespace}/{repository}/archive",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    request_body = ArchiveRequest,
    responses(
        (status = 204, description = "Archive status updated"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/archive", method = "PATCH", err = "json")]
pub(crate) async fn toggle_archive(
    repo: Repository,
    web_user: WebUser,
    body: web::Json<ArchiveRequest>,
    request: HttpRequest,
    meili_client: web::Data<MeiliClient>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut tx = db_pool.begin().await?;

    if !privilege::check_admin(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let value = if body.archive { "now()" } else { "null" };

    let repo: Repository = sqlx::query_as(&format!("update repositories set archived_at = {value} where id = $1 returning *"))
        .bind(repo.id)
        .fetch_one(&mut *tx)
        .await?;

    Event::new(
        if body.archive { "repo.archived" } else { "repo.unarchived" },
        user.id,
        &request,
        (&repo).into(),
        None,
    )
    .save(&mut tx)
    .await?;

    tx.commit().await?;

    repo.index_meili(&meili_client).await;

    Ok(HttpResponse::NoContent().finish())
}
