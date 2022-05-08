use actix_web::web::ServiceConfig;

mod api;
mod avatar;
mod sso;
mod user_create;
mod user_login;
mod user_logout;
mod user_verify;

pub(crate) fn init(config: &mut ServiceConfig) {
    api::init(config);

    config.service(user_create::get_register);
    config.service(user_create::post_register);

    config.service(user_login::get_login);
    config.service(user_login::post_login);

    config.service(user_logout::logout);
    config.service(user_verify::verify);

    config.service(avatar::get_avatar);
    config.service(avatar::put_avatar);

    config.service(sso::initiate_sso);
    config.service(sso::sso_callback);
}
