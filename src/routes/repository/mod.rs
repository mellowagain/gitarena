use actix_web::web::ServiceConfig;
use serde::Deserialize;

mod api;
mod archive;
mod blobs;
mod commits;
mod git;
mod import;
mod issues;
mod repo_create;
mod repo_view;

pub(crate) fn init(config: &mut ServiceConfig) {
    api::init(config);
    blobs::init(config);
    git::init(config); // Git smart protocol v2 routes

    config.service(commits::commits);
    config.service(archive::tar_gz_file);
    config.service(archive::zip_file);
    config.service(issues::all_issues);
    config.service(import::import_repo);
    config.service(repo_create::new_repo);
    config.service(repo_view::view_repo);
    config.service(repo_view::view_repo_tree); // Always needs to be last in this list
}

#[derive(Deserialize)]
pub(crate) struct GitRequest {
    pub(crate) username: String,
    pub(crate) repository: String,
}

#[derive(Deserialize)]
pub(crate) struct GitTreeRequest {
    pub(crate) username: String,
    pub(crate) repository: String,
    pub(crate) tree: String,
}
