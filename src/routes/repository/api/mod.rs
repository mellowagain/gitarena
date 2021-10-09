use actix_web::web::ServiceConfig;

mod create_repo;
mod repo_meta;
mod repo_readme;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(create_repo::create);
    config.service(repo_meta::meta);
    config.service(repo_readme::readme);
}
