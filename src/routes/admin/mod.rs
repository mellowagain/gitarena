use actix_web::Scope;
use actix_web::web::scope;

mod dashboard;
mod settings;

pub(crate) fn all() -> Scope {
    scope("/admin")
        .service(dashboard::dashboard)
        .service(settings::get_settings)
        .service(settings::patch_settings)
}
