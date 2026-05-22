use actix_web::web::ServiceConfig;

pub(crate) mod code;
pub(crate) mod repositories;
pub(crate) mod users;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(code::get_code_search);
    config.service(repositories::get_repo_search);
    config.service(users::get_user_search);
}
