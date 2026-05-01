use actix_web::web::ServiceConfig;
use serde::Serialize;
use utoipa::ToSchema;

pub(crate) mod branch_commits;
pub(crate) mod branch_files;
pub(crate) mod branches;
pub(crate) mod commit_detail;
pub(crate) mod create_repo;
pub(crate) mod file_content;
pub(crate) mod fork_repo;
pub(crate) mod import_repo;
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
    config.service(branch_commits::branch_commits);
    config.service(commit_detail::commit_detail);
    config.service(branch_files::branch_files);
    config.service(branches::branches);

    config.service(fork_repo::create_fork);

    config.service(star::get_stats);
    config.service(star::post_star);
    config.service(star::delete_star);
    config.service(star::put_star);
}

#[derive(Serialize, ToSchema)]
pub(crate) struct CreateJsonResponse {
    /// Internal ID of the newly created repository
    pub(crate) id: i32,
    /// Full URL to the repository
    pub(crate) url: String,
}
