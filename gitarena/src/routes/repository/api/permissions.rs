use crate::database::Pool;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::user::WebUser;

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepoPermissions {
    view: bool,
    push: bool,
    manage_issues: bool,
    admin: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PermissionsResponse {
    permissions: RepoPermissions,
}

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/permissions",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "Permission map for the current user", body = PermissionsResponse),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/permissions", method = "GET", err = "json")]
pub(crate) async fn get_permissions(repo: Repository, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut tx = db_pool.begin().await?;

    let permissions = match web_user.as_ref() {
        None => RepoPermissions {
            view: true,
            push: false,
            manage_issues: false,
            admin: false,
        },
        Some(user) => {
            let (is_owner, is_instance_admin) = (repo.owner_user == Some(user.id), user.admin);

            if is_owner || is_instance_admin {
                RepoPermissions {
                    view: true,
                    push: true,
                    manage_issues: true,
                    admin: true,
                }
            } else {
                match privilege::get_repo_privilege(&repo, user, &mut tx).await? {
                    None => RepoPermissions {
                        view: true,
                        push: false,
                        manage_issues: false,
                        admin: false,
                    },
                    Some(privilege) => RepoPermissions {
                        view: privilege.access_level.can_view(),
                        push: privilege.access_level.can_push(),
                        manage_issues: privilege.access_level.can_manage_issues(),
                        admin: privilege.access_level.can_admin(),
                    },
                }
            }
        }
    };

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(PermissionsResponse { permissions }))
}
