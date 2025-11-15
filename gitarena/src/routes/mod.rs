use actix_web::web::ServiceConfig;

pub(crate) mod admin;
mod api;
mod explore;
pub(crate) mod not_found;
pub(crate) mod proxy;
pub(crate) mod repository;
pub(crate) mod user;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(api::api);
    config.service(explore::explore);
}
