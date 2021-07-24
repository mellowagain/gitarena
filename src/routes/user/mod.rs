use actix_web::web::ServiceConfig;

mod user_create;
mod user_login;
mod user_logout;
mod user_verify;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(user_create::register);
    config.service(user_login::login);
    config.service(user_logout::logout);
    config.service(user_verify::verify);
}
