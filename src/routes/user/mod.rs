use actix_web::web::ServiceConfig;

pub(crate) mod user_create;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(user_create::register);
}
