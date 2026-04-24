use actix_web::web::ServiceConfig;

pub(crate) mod add_key;
pub(crate) mod auth;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(add_key::put_ssh_key);
    auth::init(config);
}
