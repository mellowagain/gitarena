use actix_web::web::ServiceConfig;
use serde::Deserialize;

mod create_repo;
//mod git_upload_pack;
mod info_refs;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(create_repo::create);

    // Git "Dumb Protocol" routes
    //config.service(git_upload_pack::git_upload_pack);
    config.service(info_refs::info_refs);
}

#[derive(Deserialize)]
pub(crate) struct GitRequest {
    username: String,
    repository: String
}