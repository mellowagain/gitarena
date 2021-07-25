use actix_web::web::ServiceConfig;

mod create_repo;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(create_repo::create);
}
