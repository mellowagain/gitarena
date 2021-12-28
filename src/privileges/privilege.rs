use crate::privileges::repo_access::AccessLevel;
use crate::privileges::repo_visibility::RepoVisibility;
use crate::repository::Repository;
use crate::user::User;

use anyhow::{Context, Result};
use sqlx::{Executor, FromRow, Postgres};

#[derive(FromRow)]
pub(crate) struct Privilege {
    pub(crate) id: i32,
    pub(crate) user_id: i32,
    pub(crate) repo_id: i32,
    pub(crate) access_level: AccessLevel
}

macro_rules! generate_check {
    ($name:ident, $target:ident) => {
        pub(crate) async fn $name<'e, E: Executor<'e, Database = Postgres>>(repo: &Repository, user: Option<&User>, executor: E) -> Result<bool> {
            Ok(if let Some(user) = user {
                if &user.id != &repo.owner && !user.admin {
                    get_repo_privilege(repo, user, executor)
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
    }
}

pub(crate) async fn check_access<'e, E: Executor<'e, Database = Postgres>>(repo: &Repository, user: Option<&User>, executor: E) -> Result<bool> {
    if repo.disabled {
        return Ok(user.map_or_else(|| false, |user| user.admin));
    }

    Ok(match repo.visibility {
        RepoVisibility::Private => {
            if let Some(user) = user {
                if user.id != repo.owner && !user.admin {
                    get_repo_privilege(repo, user, executor)
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
        RepoVisibility::Public => true
    })
}

generate_check!(check_manage_issues, can_manage_issues);
generate_check!(check_push, can_push);
generate_check!(check_admin, can_admin);

async fn get_repo_privilege<'e, E: Executor<'e, Database = Postgres>>(repo: &Repository, user: &User, executor: E) -> Result<Option<Privilege>> {
    Ok(sqlx::query_as::<_, Privilege>("select * from privileges where user_id = $1 and repo_id = $2 limit 1")
        .bind(&user.id)
        .bind(&repo.id)
        .fetch_optional(executor)
        .await?)
}
