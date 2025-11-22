use actix_web::Scope;
use actix_web::web::scope;

mod dashboard;
mod log;
mod settings;

pub(crate) fn all() -> Scope {
    scope("/admin")
        .service(dashboard::dashboard)
        .service(log::log)
        .service(log::log_sse)
        .service(settings::get_settings)
        .service(settings::patch_settings)
}
