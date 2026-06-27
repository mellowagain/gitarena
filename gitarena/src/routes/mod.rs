use crate::issue::IssueStatus;
use crate::organization::{OrgMember, OrgRole, Organization};
use crate::privileges::repo_access::AccessLevel;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::release::assets::{Arch, Kind, Libc, Os, ReleaseAssets};
use crate::repository::Repository;
use crate::routes::admin::health::{ComponentStatus, InstanceComponent, InstanceHealth};
use crate::routes::admin::stats::InstanceStats;
use crate::routes::admin::users::ExtendedUser;
use crate::routes::api::ApiInfoResponse;
use crate::routes::events::EventResponse;
use crate::routes::events::contributions::{ContributionDay, ContributionsResponse};
use crate::routes::explore::{ExploreRepo, ExploreResponse};
use crate::routes::organization::api::create_org::CreateOrgRequest;
use crate::routes::organization::api::members::{AddMemberRequest, OrgMemberEntry};
use crate::routes::repository::api::CreateJsonResponse;
use crate::routes::repository::api::archive::ArchiveRequest;
use crate::routes::repository::api::blame::{BlameHunk, BlameResponse};
use crate::routes::repository::api::branch_files::{BranchFilesResponse, FileCommitInfo, FileEntry, FileType};
use crate::routes::repository::api::collaborators::{CollaboratorResponse, UpsertCollaboratorRequest};
use crate::routes::repository::api::commit_detail::{
    CommitDetailResponse, CommitMeta, DiffFile, DiffHunk, DiffLineEntry, DiffLineKind, DiffStats, DiffStatus, FileStats, SignatureInfo,
};
use crate::routes::repository::api::create_repo::CreateJsonRequest;
use crate::routes::repository::api::import_repo::ImportJsonRequest;
use crate::routes::repository::api::issues::labels::{CreateLabelRequest, LabelEntry, LabelsResponse, UpdateLabelRequest};
use crate::routes::repository::api::issues::milestones::{CreateMilestoneRequest, MilestoneEntry, MilestonesResponse, UpdateMilestoneRequest};
use crate::routes::repository::api::issues::timeline::TimelineEvent;
use crate::routes::repository::api::permissions::{PermissionsResponse, RepoPermissions};
use crate::routes::repository::api::releases::{CreateAssetRequest, CreateAssetResponse, CreateReleaseRequest, ReleaseResponse, UpdateReleaseRequest};
use crate::routes::repository::api::repo_readme::ReadmeResponse;
use crate::routes::repository::api::star::{RepoStatsDetailResponse, RepoStatsStarsResponse};
use crate::routes::repository::api::tags::{TagInfo, TagKind, TagsResponse};
use crate::routes::search::code::CodeSearchResponse;
use crate::routes::search::repositories::RepoSearchResponse;
use crate::routes::search::users::{SearchUser, UserSearchResponse};
use crate::routes::user::api::add_key::{AddKeyJsonRequest, AddKeyJsonResponse};
use crate::routes::user::api::auth::login::LoginJsonRequest;
use crate::routes::user::api::auth::me::MeResponse;
use crate::routes::user::api::issues::AssignedIssueEntry;
use crate::routes::user::api::profile::{UserProfileRepo, UserProfileResponse, UserProfileStats};
use crate::routes::user::api::sso::SSOProvidersResponse;

use actix_web::web::ServiceConfig;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};

pub(crate) mod admin;
pub(crate) mod api;
pub(crate) mod events;
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
        crate::routes::repository::api::archive::toggle_archive,
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
        crate::routes::user::api::issues::get_assigned_issues,
        crate::routes::organization::api::create_org::create_org,
        crate::routes::organization::api::org::get_org,
        crate::routes::organization::api::org::delete_org,
        crate::routes::organization::api::members::list_members,
        crate::routes::organization::api::members::add_member,
        crate::routes::organization::api::members::remove_member,
        crate::routes::admin::stats::get_instance_stats,
        crate::routes::admin::health::get_instance_health,
        crate::routes::admin::users::get_instance_users,
        crate::routes::admin::audit_log::get_admin_audit_log,
        crate::routes::events::audit_log::get_personal_audit_log,
        crate::routes::events::user_feed::get_user_feed,
        crate::routes::events::contributions::get_contributions,
        crate::routes::events::org_audit_log::get_org_audit_log,
        crate::routes::events::dashboard::get_dashboard_feed,
        crate::routes::search::code::get_code_search,
        crate::routes::search::repositories::get_repo_search,
        crate::routes::search::users::get_user_search,
        crate::routes::repository::api::collaborators::list_collaborators,
        crate::routes::repository::api::collaborators::upsert_collaborator,
        crate::routes::repository::api::collaborators::remove_collaborator,
        crate::routes::repository::api::permissions::get_permissions,
        crate::routes::repository::api::issues::labels::list_labels,
        crate::routes::repository::api::issues::labels::create_label,
        crate::routes::repository::api::issues::labels::update_label,
        crate::routes::repository::api::issues::labels::delete_label,
        crate::routes::repository::api::issues::milestones::list_milestones,
        crate::routes::repository::api::issues::milestones::create_milestone,
        crate::routes::repository::api::issues::milestones::update_milestone,
        crate::routes::repository::api::issues::milestones::delete_milestone,
        crate::routes::repository::api::issues::timeline::get_issue_timeline,
        crate::routes::repository::api::tags::list_tags,
        crate::routes::repository::api::tags::delete_tag,
        crate::routes::repository::api::releases::list_releases,
        crate::routes::repository::api::releases::latest_release,
        crate::routes::repository::api::releases::get_release,
        crate::routes::repository::api::releases::create_release,
        crate::routes::repository::api::releases::update_release,
        crate::routes::repository::api::releases::delete_release,
        crate::routes::repository::api::releases::create_asset,
        crate::routes::repository::api::releases::confirm_asset,
        crate::routes::repository::api::releases::download_asset,
        crate::routes::repository::api::releases::delete_asset,
    ),
    components(schemas(
        ArchiveRequest,
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
        AssignedIssueEntry,
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
        EventResponse,
        ContributionsResponse,
        ContributionDay,
        CodeSearchResponse,
        RepoSearchResponse,
        UserSearchResponse,
        SearchUser,
        AccessLevel,
        CollaboratorResponse,
        UpsertCollaboratorRequest,
        PermissionsResponse,
        RepoPermissions,
        LabelEntry,
        LabelsResponse,
        CreateLabelRequest,
        UpdateLabelRequest,
        MilestoneEntry,
        MilestonesResponse,
        CreateMilestoneRequest,
        UpdateMilestoneRequest,
        IssueStatus,
        TimelineEvent,
        TagInfo,
        TagKind,
        TagsResponse,
        ReleaseResponse,
        CreateAssetResponse,
        CreateReleaseRequest,
        UpdateReleaseRequest,
        CreateAssetRequest,
        ReleaseAssets,
        Os,
        Arch,
        Libc,
        Kind,
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
