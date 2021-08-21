use actix_web::web::ServiceConfig;
use serde::Deserialize;

mod create_repo;
mod git_receive_pack;
mod git_upload_pack;
mod info_refs;
mod repo_meta;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(create_repo::create);
    config.service(repo_meta::meta);

    // Git smart protocol v2 routes
    config.service(git_receive_pack::git_receive_pack); // git push
    config.service(git_upload_pack::git_upload_pack); // git pull
    config.service(info_refs::info_refs);
}

#[derive(Deserialize)]
pub(crate) struct GitRequest {
    username: String,
    repository: String
}
