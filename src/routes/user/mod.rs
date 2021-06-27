use actix_web::web::ServiceConfig;

pub(crate) mod user_create;
pub(crate) mod user_verify;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(user_create::handle_post);
    config.service(user_verify::handle_get);
}
