use actix_web::web::ServiceConfig;

pub(crate) mod create_org;
pub(crate) mod members;
pub(crate) mod org;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(create_org::create_org);

    config.service(org::get_org);
    config.service(org::delete_org);

    config.service(members::list_members);
    config.service(members::add_member);
    config.service(members::remove_member);
}
