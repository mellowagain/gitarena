use actix_web::web::ServiceConfig;

mod create_repo;
mod repo_meta;
mod repo_readme;
mod star;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(create_repo::create);
    config.service(repo_meta::meta);
    config.service(repo_readme::readme);

    config.service(star::get_star);
    config.service(star::post_star);
    config.service(star::delete_star);
    config.service(star::put_star);
}
