use crate::database::Database;
use crate::die;
use crate::events::Subject;
use crate::organization::{OrgMember, OrgRole};
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::token::{Token, TokenPermission, TokenScope, TokenType};
use crate::user::User;
use actix_web::web::ServiceConfig;

use anyhow::Result;
use serde::Serialize;
use sqlx::{FromRow, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

pub(crate) mod create;
pub(crate) mod list;
pub(crate) mod permissions;
pub(crate) mod revoke;
pub(crate) mod update;

const TOKEN_SELECT: &str = "select t.*, \
                            coalesce(array_agg(s.scope_org) filter (where s.scope_org is not null), '{}'::uuid[]) as scope_orgs, \
                            coalesce(array_agg(s.scope_repo) filter (where s.scope_repo is not null), '{}'::uuid[]) as scope_repos \
                            from tokens t \
                            left join token_scopes s on s.token_id = t.id";

pub(crate) fn init(config: &mut ServiceConfig) {
    config
        .service(permissions::get_token_permissions)
        .service(list::list_tokens)
        .service(list::get_token)
        .service(create::create_token)
        .service(update::update_token)
        .service(revoke::revoke_token);
}

#[derive(FromRow, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenResponse {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub(crate) token: Token,
    /// UUIDs of the organizations this token is limited to
    pub(crate) scope_orgs: Vec<Uuid>,
    /// UUIDs of the repositories this token is limited to
    pub(crate) scope_repos: Vec<Uuid>,
}

pub(crate) fn scope_allowed(token_type: TokenType, scope: TokenScope) -> bool {
    match token_type {
        TokenType::Personal => true,
        TokenType::Organization => matches!(scope, TokenScope::All | TokenScope::Selected),
        _ => scope == TokenScope::All,
    }
}

async fn is_org_admin(org: Uuid, user: &User, tx: &mut Transaction<'_, Database>) -> Result<bool> {
    Ok(OrgMember::get_role(org, user.id, tx)
        .await?
        .is_some_and(|role| OrgMember::has_permission(role, OrgRole::Admin)))
}

pub(crate) async fn can_administer(
    owner_user: Option<Uuid>,
    owner_org: Option<Uuid>,
    owner_repo: Option<Uuid>,
    user: &User,
    tx: &mut Transaction<'_, Database>,
) -> Result<bool> {
    match (owner_user, owner_org, owner_repo) {
        (Some(owner), None, None) => Ok(owner == user.id),
        (None, Some(org), None) => Ok(user.admin || is_org_admin(org, user, tx).await?),
        (None, None, Some(repo)) => match Repository::find_by_id(repo, tx).await {
            Some(repo) => {
                if privilege::check_admin(&repo, Some(user), tx).await? {
                    return Ok(true);
                }

                match repo.owner_org {
                    Some(org) => is_org_admin(org, user, tx).await,
                    None => Ok(false),
                }
            }
            None => Ok(false),
        },
        (None, None, None) => Ok(user.admin),
        _ => Ok(false),
    }
}

pub(crate) fn check_permissions(permissions: &[TokenPermission], existing: &[TokenPermission], user: &User) -> Result<()> {
    if user.admin {
        return Ok(());
    }

    if permissions
        .iter()
        .any(|permission| permission.requires_instance_admin() && !existing.contains(permission))
    {
        die!(FORBIDDEN, "Only instance admins can grant admin permissions");
    }

    Ok(())
}

pub(crate) async fn check_scope_targets(
    token_type: TokenType,
    scope: TokenScope,
    owner_org: Option<Uuid>,
    scope_orgs: &[Uuid],
    scope_repos: &[Uuid],
    user: &User,
    tx: &mut Transaction<'_, Database>,
) -> Result<()> {
    if scope != TokenScope::Selected {
        if !scope_orgs.is_empty() || !scope_repos.is_empty() {
            die!(BAD_REQUEST, "Scope targets can only be set on tokens using the selected scope");
        }

        return Ok(());
    }

    if scope_orgs.is_empty() && scope_repos.is_empty() {
        die!(BAD_REQUEST, "Selected scope requires at least one organization or repository");
    }

    if token_type == TokenType::Organization && !scope_orgs.is_empty() {
        die!(BAD_REQUEST, "Organization tokens can only be limited to repositories");
    }

    let unreachable_org: Option<Uuid> = sqlx::query_scalar(
        "select t.id from unnest($1::uuid[]) as t(id) \
         where not $3 and not exists (select 1 from organization_members m where m.org_id = t.id and m.user_id = $2) limit 1",
    )
    .bind(scope_orgs)
    .bind(user.id)
    .bind(user.admin)
    .fetch_optional(&mut **tx)
    .await?;

    if unreachable_org.is_some() {
        die!(BAD_REQUEST, "Scope targets can only contain organizations you are a member of");
    }

    for target in scope_repos {
        let Some(repo) = Repository::find_by_id(*target, tx).await else {
            die!(BAD_REQUEST, "Unknown repository in scope targets");
        };

        if let Some(org) = owner_org {
            if repo.owner_org != Some(org) {
                die!(BAD_REQUEST, "Organization tokens can only be limited to repositories owned by the organization");
            }
        } else if !privilege::check_access(&repo, Some(user), tx).await? {
            die!(BAD_REQUEST, "Scope targets can only contain repositories you have access to");
        }
    }

    Ok(())
}

pub(crate) async fn insert_scope_targets(
    token_id: Uuid,
    token_type: TokenType,
    scope_orgs: &[Uuid],
    scope_repos: &[Uuid],
    tx: &mut Transaction<'_, Database>,
) -> Result<()> {
    sqlx::query("insert into token_scopes (token_id, token_type, scope_org) select distinct $1::uuid, $2::token_type, t.id from unnest($3::uuid[]) as t(id)")
        .bind(token_id)
        .bind(token_type)
        .bind(scope_orgs)
        .execute(&mut **tx)
        .await?;

    sqlx::query("insert into token_scopes (token_id, token_type, scope_repo) select distinct $1::uuid, $2::token_type, t.id from unnest($3::uuid[]) as t(id)")
        .bind(token_id)
        .bind(token_type)
        .bind(scope_repos)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

pub(crate) async fn find_token(id: Uuid, tx: &mut Transaction<'_, Database>) -> Result<Option<TokenResponse>> {
    Ok(sqlx::query_as::<_, TokenResponse>(&format!("{TOKEN_SELECT} where t.id = $1 group by t.id"))
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?)
}

pub(crate) async fn name_taken(
    name: &str,
    token: Option<Uuid>,
    owner: (Option<Uuid>, Option<Uuid>, Option<Uuid>),
    tx: &mut Transaction<'_, Database>,
) -> Result<bool> {
    let (owner_user, owner_org, owner_repo) = owner;

    let exists: bool = sqlx::query_scalar(
        "select exists(select 1 from tokens where name = $1 and id is distinct from $2 and owner_user is not distinct from $3 \
         and owner_org is not distinct from $4 and owner_repo is not distinct from $5 and revoked_at is null limit 1)",
    )
    .bind(name)
    .bind(token)
    .bind(owner_user)
    .bind(owner_org)
    .bind(owner_repo)
    .fetch_one(&mut **tx)
    .await?;

    Ok(exists)
}

pub(crate) fn subject_of(token: &Token, actor: &User) -> Subject {
    match (token.owner_org, token.owner_repo) {
        (Some(org), _) => Subject::Org(org),
        (_, Some(repo)) => Subject::Repo(repo),
        _ => Subject::User(token.owner_user.unwrap_or(actor.id)),
    }
}
