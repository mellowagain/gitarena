use crate::database::Pool;
use crate::die;
use crate::events::{Event, Subject};
use crate::meili::MeiliClient;
use crate::organization::Organization;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::repository::cleanup::RepoCleanup;
use crate::user::WebUser;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::{Result, anyhow};
use gitarena_macros::route;
use serde_json::json;

#[utoipa::path(
    delete,
    path = "/api/repos/{namespace}/{repository}",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 204, description = "Repository deleted"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}", method = "DELETE", err = "json")]
pub(crate) async fn delete_repo(
    repo: Repository,
    web_user: WebUser,
    request: HttpRequest,
    meili_client: web::Data<MeiliClient>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let mut tx = db_pool.begin().await?;

    if !privilege::check_delete(&repo, &user, &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let (subject, namespace) = if let Some(org_id) = repo.owner_org {
        let org = Organization::find_by_id(org_id, &mut tx)
            .await
            .ok_or_else(|| anyhow!("owning org of repo {} not found", repo.id))?;

        (Subject::Org(org_id), org.name)
    } else {
        (Subject::User(user.id), user.username.clone())
    };

    let path = repo.get_fs_path(&mut tx).await?;

    let mut cleanup = RepoCleanup::prepare(vec![repo.id], &mut tx).await?;

    Event::new(
        "repo.deleted",
        user.id,
        &request,
        subject,
        Some(json!({
            "id": repo.id,
            "namespace": namespace,
            "name": repo.name,
        })),
    )
    .save(&mut tx)
    .await?;

    sqlx::query("delete from repositories where id = $1").bind(repo.id).execute(&mut *tx).await?;

    cleanup.move_to_trash(path, repo.id, &mut tx).await?;
    cleanup.commit(tx, &meili_client).await?;

    Ok(HttpResponse::NoContent().finish())
}
