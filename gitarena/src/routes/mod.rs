use crate::organization::{OrgMember, OrgRole, Organization};
use crate::privileges::repo_access::AccessLevel;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::routes::admin::health::{ComponentStatus, InstanceComponent, InstanceHealth};
use crate::routes::admin::stats::InstanceStats;
use crate::routes::admin::users::ExtendedUser;
use crate::routes::api::ApiInfoResponse;
use crate::routes::explore::{ExploreRepo, ExploreResponse};
use crate::routes::organization::api::create_org::CreateOrgRequest;
use crate::routes::organization::api::members::{AddMemberRequest, OrgMemberEntry};
use crate::routes::repository::api::CreateJsonResponse;
use crate::routes::repository::api::blame::{BlameHunk, BlameResponse};
use crate::routes::repository::api::branch_files::{BranchFilesResponse, FileCommitInfo, FileEntry, FileType};
use crate::routes::repository::api::collaborators::{CollaboratorResponse, UpsertCollaboratorRequest};
use crate::routes::repository::api::commit_detail::{
    CommitDetailResponse, CommitMeta, DiffFile, DiffHunk, DiffLineEntry, DiffLineKind, DiffStats, DiffStatus, FileStats, SignatureInfo,
};
use crate::routes::repository::api::create_repo::CreateJsonRequest;
use crate::routes::repository::api::import_repo::ImportJsonRequest;
use crate::routes::repository::api::repo_readme::ReadmeResponse;
use crate::routes::repository::api::star::{RepoStatsDetailResponse, RepoStatsStarsResponse};
use crate::routes::search::code::CodeSearchResponse;
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
pub(crate) mod organization;
pub(crate) mod proxy;
pub(crate) mod repository;
pub(crate) mod search;
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
        crate::routes::repository::api::blame::blame,
        crate::routes::repository::api::commit_detail::commit_detail,
        crate::routes::user::api::add_key::put_ssh_key,
        crate::routes::user::api::sso::get_sso_providers,
        crate::routes::user::api::auth::login::post_login,
        crate::routes::user::api::auth::logout::post_logout,
        crate::routes::user::api::auth::me::get_me,
        crate::routes::user::api::profile::get_user_profile,
        crate::routes::organization::api::create_org::create_org,
        crate::routes::organization::api::org::get_org,
        crate::routes::organization::api::org::delete_org,
        crate::routes::organization::api::members::list_members,
        crate::routes::organization::api::members::add_member,
        crate::routes::organization::api::members::remove_member,
        crate::routes::admin::stats::get_instance_stats,
        crate::routes::admin::health::get_instance_health,
        crate::routes::admin::users::get_instance_users,
        crate::routes::search::code::get_code_search,
        crate::routes::repository::api::collaborators::list_collaborators,
        crate::routes::repository::api::collaborators::upsert_collaborator,
        crate::routes::repository::api::collaborators::remove_collaborator,
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
        BlameResponse,
        BlameHunk,
        CommitDetailResponse,
        CommitMeta,
        SignatureInfo,
        DiffStats,
        FileStats,
        DiffLineKind,
        DiffLineEntry,
        DiffHunk,
        DiffStatus,
        DiffFile,
        AddKeyJsonRequest,
        AddKeyJsonResponse,
        SSOProvidersResponse,
        LoginJsonRequest,
        MeResponse,
        UserProfileResponse,
        UserProfileRepo,
        UserProfileStats,
        Organization,
        OrgRole,
        OrgMember,
        CreateOrgRequest,
        AddMemberRequest,
        OrgMemberEntry,
        InstanceStats,
        InstanceHealth,
        InstanceComponent,
        ComponentStatus,
        ExtendedUser,
        CodeSearchResponse,
        AccessLevel,
        CollaboratorResponse,
        UpsertCollaboratorRequest,
    )),
    modifiers(&CookieAuth),
    tags(
        (name = "api", description = "General API endpoints"),
        (name = "explore", description = "Explore public repositories"),
        (name = "repository", description = "Repository management"),
        (name = "organization", description = "Organization management"),
        (name = "user", description = "User account management"),
        (name = "search", description = "Search endpoints"),
        (name = "admin", description = "Instance management for admins"),
    )
)]
pub(crate) struct ApiDoc;
