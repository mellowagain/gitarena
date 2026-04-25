use actix_web::web::ServiceConfig;

pub(crate) mod add_key;
pub(crate) mod auth;
pub(crate) mod sso;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(add_key::put_ssh_key);
    auth::init(config);

    config.service(sso::get_sso_providers);
    config.service(sso::initiate_sso);
    config.service(sso::sso_callback);
}
