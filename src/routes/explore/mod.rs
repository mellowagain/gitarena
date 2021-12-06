use actix_web::web::ServiceConfig;

mod explore_repos;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(explore_repos::explore);
}
