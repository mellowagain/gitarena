use actix_web::web::ServiceConfig;

pub(crate) mod login;
pub(crate) mod logout;
pub(crate) mod me;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(login::post_login);
    config.service(logout::post_logout);
    config.service(me::get_me);
}
