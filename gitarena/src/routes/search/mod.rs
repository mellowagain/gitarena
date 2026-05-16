use actix_web::web::ServiceConfig;

pub(crate) mod code;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(code::get_code_search);
}
