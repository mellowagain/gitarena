use actix_web::web::ServiceConfig;

pub(crate) mod health;
pub(crate) mod stats;
pub(crate) mod users;

pub(crate) fn init(config: &mut ServiceConfig) {
    config
        .service(stats::get_instance_stats)
        .service(health::get_instance_health)
        .service(users::get_instance_users);
}
