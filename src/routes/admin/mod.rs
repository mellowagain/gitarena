use actix_web::Scope;
use actix_web::web::scope;

mod dashboard;

pub(crate) fn all() -> Scope {
    scope("/admin")
        .service(dashboard::dashboard)
}
