use actix_web::web::ServiceConfig;

pub(crate) mod api;

pub(crate) fn init(config: &mut ServiceConfig) {
    api::init(config);
}
