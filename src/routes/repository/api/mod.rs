use actix_web::web::ServiceConfig;
use serde::Serialize;

mod create_repo;
mod fork_repo;
mod import_repo;
mod repo_meta;
mod repo_readme;
mod star;

pub(crate) fn init(config: &mut ServiceConfig) {
    // import_repo needs to be always above create_repo
    config.service(import_repo::import);
    config.service(create_repo::create);
    config.service(repo_meta::meta);
    config.service(repo_readme::readme);

    config.service(fork_repo::get_fork_amount);
    config.service(fork_repo::create_fork);

    config.service(star::get_star);
    config.service(star::post_star);
    config.service(star::delete_star);
    config.service(star::put_star);
}

#[derive(Serialize)]
pub(crate) struct CreateJsonResponse {
    pub(crate) id: i32,
    pub(crate) url: String,
}
