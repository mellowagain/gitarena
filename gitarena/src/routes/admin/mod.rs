use actix_web::web::ServiceConfig;

pub(crate) mod audit_log;
pub(crate) mod health;
pub(crate) mod stats;
pub(crate) mod update_check;
pub(crate) mod users;

pub(crate) fn init(config: &mut ServiceConfig) {
    config
        .service(audit_log::get_admin_audit_log)
        .service(stats::get_instance_stats)
        .service(health::get_instance_health)
        .service(update_check::get_update_check)
        .service(users::get_instance_users)
        .service(users::delete_instance_user);
}
