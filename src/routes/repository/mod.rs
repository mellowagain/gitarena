use actix_web::web::ServiceConfig;

mod create_repo;
mod repo_git;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(create_repo::create);

    config.service(repo_git::info_refs);
}
