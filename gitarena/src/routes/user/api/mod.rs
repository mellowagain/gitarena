use actix_web::web::ServiceConfig;

pub(crate) mod add_key;
pub(crate) mod auth;
pub(crate) mod emails;
pub(crate) mod orgs;
pub(crate) mod profile;
pub(crate) mod sessions;
pub(crate) mod ssh_keys;
pub(crate) mod sso;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(add_key::put_ssh_key);
    config.service(profile::get_user_profile);
    config.service(profile::get_user_by_id);
    config.service(orgs::get_user_orgs);
    auth::init(config);

    config.service(emails::get_emails);
    config.service(emails::post_email);
    config.service(emails::delete_email);
    config.service(emails::patch_email);
    config.service(emails::resend_verify_email);

    config.service(sessions::get_sessions);
    config.service(sessions::delete_session);
    config.service(sessions::delete_all_sessions);

    config.service(ssh_keys::get_ssh_keys);
    config.service(ssh_keys::delete_ssh_key);

    config.service(sso::get_sso_providers);
    config.service(sso::initiate_sso);
    config.service(sso::sso_callback);
}
