use actix_web::web::ServiceConfig;
use serde::Deserialize;

mod archive;
mod create_repo;
mod git_receive_pack;
mod git_upload_pack;
mod info_refs;
mod repo_meta;
mod repo_readme;
mod repo_view;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(archive::tar_gz_file);
    config.service(archive::zip_file);
    config.service(create_repo::create);
    config.service(repo_meta::meta);
    config.service(repo_readme::readme);
    config.service(repo_view::view_repo);
    config.service(repo_view::view_repo_tree);

    // Git smart protocol v2 routes
    config.service(git_receive_pack::git_receive_pack); // git push
    config.service(git_upload_pack::git_upload_pack); // git pull
    config.service(info_refs::info_refs);
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
