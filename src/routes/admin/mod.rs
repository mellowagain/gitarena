use actix_web::web::ServiceConfig;

mod dashboard;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(dashboard::admin_dashboard);
}
