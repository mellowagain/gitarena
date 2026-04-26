use actix_web::web::ServiceConfig;

pub(crate) mod login;
pub(crate) mod logout;
pub(crate) mod me;
pub(crate) mod passkey;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(login::post_login);
    config.service(logout::post_logout);
    config.service(me::get_me);
    config.service(passkey::post_register_start);
    config.service(passkey::post_register_finish);
    config.service(passkey::post_login_start);
    config.service(passkey::post_login_finish);
    config.service(passkey::get_passkeys);
    config.service(passkey::delete_passkey);
}
