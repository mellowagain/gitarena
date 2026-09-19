use crate::crypto;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use derive_more::Display;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::{FromRow, Type};
use strum::VariantArray;
use utoipa::ToSchema;
use uuid::Uuid;

const SECRET_LENGTH: usize = 128;

#[derive(FromRow, Display, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[display("{name}")]
pub(crate) struct Token {
    /// ID
    pub(crate) id: Uuid,
    /// Name
    pub(crate) name: String,
    #[serde(skip)]
    #[allow(clippy::struct_field_names)]
    pub(crate) token: Vec<u8>,
    /// UUID of the user who owns this token
    /// Only set for `token_type` = `personal`
    pub(crate) owner_user: Option<Uuid>,
    /// UUID of the organization that owns this token
    /// Only set for `token_type` = `organization`
    pub(crate) owner_org: Option<Uuid>,
    /// UUID of the repository that owns this token.
    /// Only set for `token_type` = \[`ci`, `deploy`, `runner?`]
    pub(crate) owner_repo: Option<Uuid>,
    /// UUID of the user that created this token. Not used for scoping
    /// Unset once the creating user has been deleted
    pub(crate) creator: Option<Uuid>,
    /// Type
    #[allow(clippy::struct_field_names)]
    pub(crate) token_type: TokenType,
    /// Scope
    pub(crate) scope: TokenScope,
    /// Permissions
    pub(crate) permissions: Vec<TokenPermission>,
    /// Last used
    pub(crate) last_used_at: Option<DateTime<Utc>>,
    /// Expiration
    pub(crate) expires_at: Option<DateTime<Utc>>,
    /// Revocation
    pub(crate) revoked_at: Option<DateTime<Utc>>,
}

impl Token {
    /// Generates a new token secret and the hmac stored in the database.
    /// The secret is only ever shown to the user once, right after creation
    pub(crate) fn generate_secret(token_type: TokenType, key: &str) -> Result<(String, Vec<u8>)> {
        let secret = format!("sk_ga{}_{}", token_type.letter(), crypto::random_hex_string(SECRET_LENGTH)?);
        let hmac = Token::hmac(key, &secret)?;

        Ok((secret, hmac))
    }

    /// Derives the hmac-sha256 a token is stored and looked up as
    pub(crate) fn hmac(key: &str, secret: &str) -> Result<Vec<u8>> {
        let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).context("failed to create hmac from token key")?;
        mac.update(secret.as_bytes());

        Ok(mac.finalize().into_bytes().to_vec())
    }
}

#[derive(Type, Display, Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[sqlx(type_name = "token_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[display(rename_all = "lowercase")]
pub(crate) enum TokenType {
    /// PAT
    Personal,
    /// Organization token
    Organization,
    /// Temporary token given to CI workflows
    Ci,
    /// Repo-specific deployment token
    Deploy,
    /// Token used to register CI runners
    Runner,
    /// Instance-wide admin token
    Instance,
}

impl TokenType {
    /// Letter identifying the type in the token prefix, e.g. `sk_gap_` for personal tokens
    fn letter(self) -> char {
        match self {
            TokenType::Personal => 'p',
            TokenType::Organization => 'o',
            TokenType::Ci => 'c',
            TokenType::Deploy => 'd',
            TokenType::Runner => 'r',
            TokenType::Instance => 'i',
        }
    }
}

#[derive(Type, Display, Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[sqlx(type_name = "token_scope", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[display(rename_all = "lowercase")]
pub(crate) enum TokenScope {
    /// Every resource the owner can access
    All,
    /// Only public resources
    /// Only allowed for `token_type` = `personal`
    Public,
    /// Only the organizations and repositories listed in `token_scopes`
    /// Only allowed for `token_type` = \[`personal`, `organization`]
    Selected,
}

#[derive(Type, Display, Debug, Clone, PartialEq, Eq, Deserialize, Serialize, ToSchema, VariantArray)]
#[sqlx(type_name = "token_permissions")]
pub(crate) enum TokenPermission {
    #[sqlx(rename = "contents:read")]
    #[serde(rename = "contents:read")]
    #[display("contents:read")]
    ContentsRead,
    #[sqlx(rename = "contents:write")]
    #[serde(rename = "contents:write")]
    #[display("contents:write")]
    ContentsWrite,

    #[sqlx(rename = "repo:read")]
    #[serde(rename = "repo:read")]
    #[display("repo:read")]
    RepoRead,
    #[sqlx(rename = "repo:write")]
    #[serde(rename = "repo:write")]
    #[display("repo:write")]
    RepoWrite,
    #[sqlx(rename = "repo:create")]
    #[serde(rename = "repo:create")]
    #[display("repo:create")]
    RepoCreate,
    #[sqlx(rename = "repo:delete")]
    #[serde(rename = "repo:delete")]
    #[display("repo:delete")]
    RepoDelete,
    #[sqlx(rename = "repo:admin")]
    #[serde(rename = "repo:admin")]
    #[display("repo:admin")]
    RepoAdmin,

    #[sqlx(rename = "releases:read")]
    #[serde(rename = "releases:read")]
    #[display("releases:read")]
    ReleasesRead,
    #[sqlx(rename = "releases:write")]
    #[serde(rename = "releases:write")]
    #[display("releases:write")]
    ReleasesWrite,
    #[sqlx(rename = "releases:assets:write")]
    #[serde(rename = "releases:assets:write")]
    #[display("releases:assets:write")]
    ReleasesAssetsWrite,

    #[sqlx(rename = "issues:read")]
    #[serde(rename = "issues:read")]
    #[display("issues:read")]
    IssuesRead,
    #[sqlx(rename = "issues:write")]
    #[serde(rename = "issues:write")]
    #[display("issues:write")]
    IssuesWrite,
    #[sqlx(rename = "issues:comment")]
    #[serde(rename = "issues:comment")]
    #[display("issues:comment")]
    IssuesComment,
    #[sqlx(rename = "issues:labels")]
    #[serde(rename = "issues:labels")]
    #[display("issues:labels")]
    IssuesLabels,
    #[sqlx(rename = "issues:milestones")]
    #[serde(rename = "issues:milestones")]
    #[display("issues:milestones")]
    IssuesMilestones,

    #[sqlx(rename = "org:read")]
    #[serde(rename = "org:read")]
    #[display("org:read")]
    OrgRead,
    #[sqlx(rename = "org:write")]
    #[serde(rename = "org:write")]
    #[display("org:write")]
    OrgWrite,
    #[sqlx(rename = "org:create")]
    #[serde(rename = "org:create")]
    #[display("org:create")]
    OrgCreate,
    #[sqlx(rename = "org:delete")]
    #[serde(rename = "org:delete")]
    #[display("org:delete")]
    OrgDelete,
    #[sqlx(rename = "org:members:read")]
    #[serde(rename = "org:members:read")]
    #[display("org:members:read")]
    OrgMembersRead,
    #[sqlx(rename = "org:members:write")]
    #[serde(rename = "org:members:write")]
    #[display("org:members:write")]
    OrgMembersWrite,
    #[sqlx(rename = "org:audit_log:read")]
    #[serde(rename = "org:audit_log:read")]
    #[display("org:audit_log:read")]
    OrgAuditLogRead,

    #[sqlx(rename = "user:read")]
    #[serde(rename = "user:read")]
    #[display("user:read")]
    UserRead,
    #[sqlx(rename = "user:write")]
    #[serde(rename = "user:write")]
    #[display("user:write")]
    UserWrite,
    #[sqlx(rename = "user:email:read")]
    #[serde(rename = "user:email:read")]
    #[display("user:email:read")]
    UserEmailRead,

    #[sqlx(rename = "ssh_keys:read")]
    #[serde(rename = "ssh_keys:read")]
    #[display("ssh_keys:read")]
    SshKeysRead,

    #[sqlx(rename = "sessions:read")]
    #[serde(rename = "sessions:read")]
    #[display("sessions:read")]
    SessionsRead,

    #[sqlx(rename = "stars:read")]
    #[serde(rename = "stars:read")]
    #[display("stars:read")]
    StarsRead,
    #[sqlx(rename = "stars:write")]
    #[serde(rename = "stars:write")]
    #[display("stars:write")]
    StarsWrite,

    #[sqlx(rename = "events:read")]
    #[serde(rename = "events:read")]
    #[display("events:read")]
    EventsRead,

    #[sqlx(rename = "audit_log:read")]
    #[serde(rename = "audit_log:read")]
    #[display("audit_log:read")]
    AuditLogRead,

    #[sqlx(rename = "admin:users:read")]
    #[serde(rename = "admin:users:read")]
    #[display("admin:users:read")]
    AdminUsersRead,
    #[sqlx(rename = "admin:users:write")]
    #[serde(rename = "admin:users:write")]
    #[display("admin:users:write")]
    AdminUsersWrite,
    #[sqlx(rename = "admin:stats:read")]
    #[serde(rename = "admin:stats:read")]
    #[display("admin:stats:read")]
    AdminStatsRead,
    #[sqlx(rename = "admin:health:read")]
    #[serde(rename = "admin:health:read")]
    #[display("admin:health:read")]
    AdminHealthRead,
    #[sqlx(rename = "admin:audit_log:read")]
    #[serde(rename = "admin:audit_log:read")]
    #[display("admin:audit_log:read")]
    AdminAuditLogRead,
    #[sqlx(rename = "admin:settings:read")]
    #[serde(rename = "admin:settings:read")]
    #[display("admin:settings:read")]
    AdminSettingsRead,
    #[sqlx(rename = "admin:settings:write")]
    #[serde(rename = "admin:settings:write")]
    #[display("admin:settings:write")]
    AdminSettingsWrite,
}

impl TokenPermission {
    pub(crate) fn grantable_permissions(admin: bool) -> Vec<String> {
        TokenPermission::VARIANTS
            .iter()
            .filter(|permission| admin || !permission.requires_instance_admin())
            .map(ToString::to_string)
            .collect()
    }

    pub(crate) fn requires_instance_admin(&self) -> bool {
        matches!(
            self,
            TokenPermission::AdminUsersRead
                | TokenPermission::AdminUsersWrite
                | TokenPermission::AdminStatsRead
                | TokenPermission::AdminHealthRead
                | TokenPermission::AdminAuditLogRead
                | TokenPermission::AdminSettingsRead
                | TokenPermission::AdminSettingsWrite
        )
    }
}
