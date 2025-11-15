use actix_web::web::ServiceConfig;

mod add_key;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(add_key::put_ssh_key);
}
