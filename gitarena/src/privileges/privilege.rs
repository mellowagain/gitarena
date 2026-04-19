use crate::privileges::repo_access::AccessLevel;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::user::User;

use anyhow::{Context, Result};
use gitarena_common::database::Database;
use sqlx::{FromRow, Transaction};
use tracing::instrument;

#[derive(FromRow, Debug)]
pub(crate) struct Privilege {
    pub(crate) id: i32,
    pub(crate) user_id: i32,
    pub(crate) repo_id: i32,
    pub(crate) access_level: AccessLevel,
}

macro_rules! generate_check {
    ($name:ident, $target:ident) => {
        #[instrument(ret, err, skip(tx))]
        pub(crate) async fn $name(repo: &Repository, user: Option<&User>, tx: &mut Transaction<'_, Database>) -> Result<bool> {
            Ok(if let Some(user) = user {
                if &user.id != &repo.owner && !user.admin {
                    get_repo_privilege(repo, user, tx)
                        .await
                        .with_context(|| format!("Unable to get repo privileges for user {} in repo {}", &user.id, &repo.id))?
                        .map_or_else(|| false, |privilege| privilege.access_level.$target())
                } else {
                    true
                }
            } else {
                false
            })
        }
    };
}

#[instrument(ret, err, skip(tx))]
pub(crate) async fn check_access(repo: &Repository, user: Option<&User>, tx: &mut Transaction<'_, Database>) -> Result<bool> {
    if repo.disabled {
        return Ok(user.map_or_else(|| false, |user| user.admin));
    }

    Ok(match repo.visibility {
        RepoVisibility::Private => {
            if let Some(user) = user {
                if user.id != repo.owner && !user.admin {
                    get_repo_privilege(repo, user, tx)
                        .await
                        .with_context(|| format!("Unable to get repo privileges for user {} in repo {}", &user.id, &repo.id))?
                        .map_or_else(|| false, |privilege| privilege.access_level.can_view())
                } else {
                    true
                }
            } else {
                false
            }
        }
        RepoVisibility::Internal => user.is_some(),
        RepoVisibility::Public => true,
    })
}

generate_check!(check_manage_issues, can_manage_issues);
generate_check!(check_push, can_push);
generate_check!(check_admin, can_admin);

#[instrument(ret, err, skip(tx))]
async fn get_repo_privilege(repo: &Repository, user: &User, tx: &mut Transaction<'_, Database>) -> Result<Option<Privilege>> {
    Ok(
        sqlx::query_as::<_, Privilege>("select * from privileges where user_id = $1 and repo_id = $2 limit 1")
            .bind(user.id)
            .bind(repo.id)
            .fetch_optional(&mut **tx)
            .await?,
    )
}
