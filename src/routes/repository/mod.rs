use actix_web::web::ServiceConfig;
use serde::Deserialize;

mod archive;
mod repo_view;
mod git;
mod api;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(archive::tar_gz_file);
    config.service(archive::zip_file);
    config.service(repo_view::view_repo);
    config.service(repo_view::view_repo_tree);

    api::init(config);
    git::init(config); // Git smart protocol v2 routes
}

#[derive(Deserialize)]
pub(crate) struct GitRequest {
    pub(crate) username: String,
    pub(crate) repository: String
}

#[derive(Deserialize)]
pub(crate) struct GitTreeRequest {
    pub(crate) username: String,
    pub(crate) repository: String,
    pub(crate) tree: String
}
