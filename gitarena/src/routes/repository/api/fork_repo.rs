use crate::config::{get_optional_setting, get_setting};
use crate::database::Pool;
use crate::die;
use crate::git::GIT_CLI_AVAILABLE;
use crate::repository::{RepoOwner, Repository};
use crate::routes::repository::api::{CreateJsonResponse, determine_namespace};
use crate::user::WebUser;
use crate::utils::filesystem::{copy_dir_all, remove_dir_on_drop};
use crate::zoekt::init_zoekt_config;

use std::mem::DropGuard;
use std::path::Path;
use std::process::Stdio;

use crate::events::Event;
use crate::meili::MeiliClient;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use anyhow::{Context, Result, anyhow};
use gitarena_issues::author::IDENTITIES_REF_PREFIX;
use gitarena_issues::bug::BUGS_REF_PREFIX;
use gitarena_macros::route;
use serde::Deserialize;
use serde_json::json;
use tokio::process::Command;
use tracing::{info, warn};
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/api/repo/{namespace}/{repository}/fork",
    summary = "Fork Repository",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    request_body = ForkJsonRequest,
    responses(
        (status = 200, description = "Fork created successfully", body = CreateJsonResponse),
        (status = 400, description = "Cannot fork into this namespace"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Repository not found or access denied"),
        (status = 409, description = "Repository name already in use in this namespace"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repo/{namespace}/{repository}/fork", method = "POST", err = "json")]
pub(crate) async fn create_fork(
    repo: Repository,
    web_user: WebUser,
    body: web::Json<ForkJsonRequest>,
    request: HttpRequest,
    meili_client: web::Data<MeiliClient>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    let (target_id, target_name) = if let Some(namespace) = &body.target_namespace {
        determine_namespace(namespace, &user, &mut tx).await?
    } else {
        (user.id, user.username.clone())
    };

    let repo_owner = repo.owner_user.or(repo.owner_org);

    if repo_owner == Some(target_id) {
        die!(BAD_REQUEST, "Cannot fork your own repository into the same namespace");
    }

    let is_org_fork = target_id != user.id;
    let owner_col = if is_org_fork { "owner_org" } else { "owner_user" };

    let (exists,): (bool,) = sqlx::query_as(&format!(
        "select exists(select 1 from repositories where {owner_col} = $1 and lower(name) = lower($2) limit 1)"
    ))
    .bind(target_id)
    .bind(&repo.name)
    .fetch_one(&mut *tx)
    .await?;

    if exists {
        die!(CONFLICT, "Repository name already in use in this namespace");
    }

    let new_repo = sqlx::query_as::<_, Repository>(&format!(
        "insert into repositories (id, {owner_col}, name, description, visibility, forked_from) values ($1, $2, $3, $4, $5, $6) returning *"
    ))
    .bind(Uuid::now_v7())
    .bind(target_id)
    .bind(&repo.name)
    .bind(&repo.description)
    .bind(repo.visibility)
    .bind(repo.id)
    .fetch_one(&mut *tx)
    .await?;

    let old_path = repo.get_fs_path(&mut tx).await?;
    let base_dir: String = get_setting("repositories.base_dir", &mut tx).await?;
    let new_path = remove_dir_on_drop(new_repo.get_fs_path(&mut tx).await?, &base_dir);

    // TODO: turn this into a task
    copy_dir_all(Path::new(old_path.as_str()), Path::new(new_path.as_str()))
        .await
        .context("Failed to copy repository")?;

    let fork_git2 = new_repo.libgit2(&mut tx).await?;

    let issue_refs = fork_git2
        .references_glob(&format!("{BUGS_REF_PREFIX}*"))?
        .chain(fork_git2.references_glob(&format!("{IDENTITIES_REF_PREFIX}*"))?)
        .collect::<Result<Vec<_>, _>>()?;

    for mut reference in issue_refs {
        reference.delete()?;
    }

    if *GIT_CLI_AVAILABLE {
        let status = Command::new("git")
            .args(["gc", "--prune=now", "--quiet"])
            .current_dir(new_path.as_str())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        match status {
            Ok(status) if !status.success() => {
                let exit_code = status.code().map_or_else(|| "unknown".to_string(), |code| code.to_string());
                warn!(exit_code, "Git garbage collector exited with non-zero status");
            }
            Ok(_) => {}
            Err(err) => warn!(?err, "Failed to execute Git garbage collector"),
        }
    }

    init_zoekt_config(&new_repo, &mut tx).await?;

    let domain: String = get_optional_setting("domain.app", &mut tx).await?.unwrap_or_default();

    Event::new(
        "repo.forked",
        user.id,
        &request,
        (&new_repo).into(),
        Some(json!({
            "forked_from": repo.id,
        })),
    )
    .save(&mut tx)
    .await?;

    tx.commit().await?;
    DropGuard::dismiss(new_path);

    repo.index_meili(&meili_client).await;

    let extensions = request.extensions();
    let repo_owner = extensions.get::<RepoOwner>().ok_or_else(|| anyhow!("Failed to lookup repo owner"))?;

    info!(
        target.id = %new_repo.id,
        target.owner = target_name,
        target.name = new_repo.name,
        source.id = %repo.id,
        source.owner = %repo_owner,
        source.name = repo.name,
        "New repository forked"
    );

    Ok(HttpResponse::Ok().json(CreateJsonResponse {
        id: new_repo.id,
        url: format!("{domain}/{target_name}/{}", new_repo.name),
    }))
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct ForkJsonRequest {
    /// Namespace (username or org name) to fork the repository into.
    /// Defaults to the authenticated user if omitted.
    #[serde(default)]
    pub(crate) target_namespace: Option<String>,
}
