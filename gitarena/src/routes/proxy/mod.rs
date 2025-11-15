use actix_web::web::ServiceConfig;

pub(crate) mod img_proxy;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(img_proxy::proxy);
}
