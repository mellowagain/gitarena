use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::routes::api::ApiInfoResponse;
use crate::routes::explore::{ExploreRepo, ExploreResponse};
use crate::routes::repository::api::CreateJsonResponse;
use crate::routes::repository::api::branch_files::{BranchFilesResponse, FileCommitInfo, FileEntry, FileType};
use crate::routes::repository::api::create_repo::CreateJsonRequest;
use crate::routes::repository::api::import_repo::ImportJsonRequest;
use crate::routes::repository::api::repo_readme::ReadmeResponse;
use crate::routes::repository::api::star::{RepoStatsDetailResponse, RepoStatsStarsResponse};
use crate::routes::user::api::add_key::{AddKeyJsonRequest, AddKeyJsonResponse};
use crate::routes::user::api::auth::login::LoginJsonRequest;
use crate::routes::user::api::auth::me::MeResponse;
use crate::routes::user::api::profile::{UserProfileRepo, UserProfileResponse, UserProfileStats};
use crate::routes::user::api::sso::SSOProvidersResponse;

use actix_web::web::ServiceConfig;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};

pub(crate) mod admin;
pub(crate) mod api;
mod explore;
pub(crate) mod not_found;
pub(crate) mod proxy;
pub(crate) mod repository;
pub(crate) mod user;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(api::api);
    config.service(explore::explore);
}

struct CookieAuth;

impl Modify for CookieAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme("cookieAuth", SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("gitarena-auth"))));
        }
    }
}

// The `OpenApi` derive macro generates internal `for_each` calls that trigger this lint.
#[allow(clippy::needless_for_each)]
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::api::api,
        crate::routes::explore::explore,
        crate::routes::repository::api::create_repo::create,
        crate::routes::repository::api::import_repo::import,
        crate::routes::repository::api::repo_meta::meta,
        crate::routes::repository::api::repo_readme::readme,
        crate::routes::repository::api::fork_repo::create_fork,
        crate::routes::repository::api::star::get_stats,
        crate::routes::repository::api::star::post_star,
        crate::routes::repository::api::star::delete_star,
        crate::routes::repository::api::branch_files::branch_files,
        crate::routes::user::api::add_key::put_ssh_key,
        crate::routes::user::api::sso::get_sso_providers,
        crate::routes::user::api::auth::login::post_login,
        crate::routes::user::api::auth::logout::post_logout,
        crate::routes::user::api::auth::me::get_me,
        crate::routes::user::api::profile::get_user_profile,
    ),
    components(schemas(
        ApiInfoResponse,
        ExploreResponse,
        ExploreRepo,
        RepoVisibility,
        Repository,
        CreateJsonRequest,
        CreateJsonResponse,
        ImportJsonRequest,
        ReadmeResponse,
        RepoStatsStarsResponse,
        RepoStatsDetailResponse,
        BranchFilesResponse,
        FileEntry,
        FileCommitInfo,
        FileType,
        AddKeyJsonRequest,
        AddKeyJsonResponse,
        SSOProvidersResponse,
        LoginJsonRequest,
        MeResponse,
        UserProfileResponse,
        UserProfileRepo,
        UserProfileStats,
    )),
    modifiers(&CookieAuth),
    tags(
        (name = "api", description = "General API endpoints"),
        (name = "explore", description = "Explore public repositories"),
        (name = "repository", description = "Repository management"),
        (name = "user", description = "User account management"),
    )
)]
pub(crate) struct ApiDoc;
