use crate::database::Database;
use crate::die;
use crate::organization::{OrgMember, Organization};
use crate::user::User;
use actix_web::web::ServiceConfig;
use anyhow::Result;
use serde::Serialize;
use sqlx::Transaction;
use utoipa::ToSchema;
use uuid::Uuid;

pub(crate) mod blame;
pub(crate) mod branch_commits;
pub(crate) mod branch_files;
pub(crate) mod branches;
pub(crate) mod collaborators;
pub(crate) mod commit_detail;
pub(crate) mod create_repo;
pub(crate) mod file_content;
pub(crate) mod fork_repo;
pub(crate) mod import_repo;
pub(crate) mod issues;
pub(crate) mod repo_meta;
pub(crate) mod repo_readme;
pub(crate) mod star;
pub(crate) mod validate_repo;

pub(crate) fn init(config: &mut ServiceConfig) {
    // import_repo and validate_repo need to be always above create_repo
    config.service(import_repo::import);
    config.service(validate_repo::validate);
    config.service(create_repo::create);
    config.service(repo_meta::meta);
    config.service(repo_readme::readme);
    config.service(file_content::file_content);
    config.service(blame::blame);
    config.service(branch_commits::branch_commits);
    config.service(commit_detail::commit_detail);
    config.service(branch_files::branch_files);
    config.service(branches::branches);

    config.service(fork_repo::create_fork);

    config.service(star::get_stats);
    config.service(star::post_star);
    config.service(star::delete_star);
    config.service(star::put_star);

    config.service(collaborators::list_collaborators);
    config.service(collaborators::upsert_collaborator);
    config.service(collaborators::remove_collaborator);

    config.service(issues::list_issues);
    config.service(issues::create_issue);
    config.service(issues::get_issue_detail);
    config.service(issues::update_issue);
    config.service(issues::delete_issue);
    config.service(issues::comments::add_issue_comment);
    config.service(issues::comments::edit_issue_comment);
    config.service(issues::comments::delete_issue_comment);
    config.service(issues::labels::list_labels);
    config.service(issues::labels::create_label);
    config.service(issues::labels::update_label);
    config.service(issues::labels::delete_label);
    config.service(issues::milestones::list_milestones);
    config.service(issues::reactions::toggle_issue_reaction);
    config.service(issues::reactions::toggle_comment_reaction);
}

pub(crate) async fn determine_namespace(namespace: &str, user: &User, tx: &mut Transaction<'_, Database>) -> Result<(Uuid, String)> {
    Ok(if namespace == user.username {
        (user.id, user.username.clone())
    } else if let Some(org) = Organization::find_by_name(namespace, tx).await {
        if OrgMember::get_role(org.id, user.id, tx).await?.is_none() {
            die!(FORBIDDEN, "You are not a member of this organization");
        }

        (org.id, org.name.clone())
    } else {
        die!(BAD_REQUEST, "Namespace not found or you do not have access to it");
    })
}

#[derive(Serialize, ToSchema)]
pub(crate) struct CreateJsonResponse {
    /// Internal ID of the newly created repository
    pub(crate) id: Uuid,
    /// Full URL to the repository
    pub(crate) url: String,
}
