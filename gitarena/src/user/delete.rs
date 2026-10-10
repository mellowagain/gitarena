use crate::config::get_setting;
use crate::database::{Database, Pool};
use crate::die;
use crate::events::{Event, SYSTEM_USER, Subject};
use crate::issue::IssueCache;
use crate::mail::Email;
use crate::mail::templates::{AccountDeletedTemplate, EmailTemplate};
use crate::meili::{ISSUES_MEILI_INDEX, MeiliClient, USERS_MEILI_INDEX};
use crate::repository::cleanup::RepoCleanup;
use crate::user::User;
use actix_web::HttpRequest;
use anyhow::{Result, anyhow};
use fang::AsyncQueue;
use serde_json::json;
use sqlx::Transaction;
use tracing::{error, instrument};
use uuid::{Uuid, uuid};

// inserted by migrations/20261010183148_deleted_user.sql
const DELETED_USER: Uuid = uuid!("01a12715-e373-78d7-ab44-172357bcbe68");

#[instrument(skip(request, tx, meili_client, db_pool, queue))]
pub(crate) async fn delete_user(
    user: &User,
    actor_id: Uuid,
    request: &HttpRequest,
    mut tx: Transaction<'_, Database>,
    meili_client: &MeiliClient,
    db_pool: &Pool,
    queue: &AsyncQueue,
) -> Result<()> {
    let system_user = *SYSTEM_USER
        .get()
        .ok_or_else(|| anyhow!("users should only be deleted after the system user has been initialized"))?;

    if user.id == system_user || user.id == DELETED_USER {
        die!(FORBIDDEN, "System users cannot be deleted");
    }

    if user.admin {
        let other_admins: i64 = sqlx::query_scalar("select count(*) from users where admin = true and id <> $1 and id <> $2")
            .bind(user.id)
            .bind(system_user)
            .fetch_one(&mut *tx)
            .await?;

        if other_admins == 0 {
            die!(CONFLICT, "Promote another user to admin before deleting the last admin account");
        }
    }

    let sole_owned_orgs: Vec<String> = sqlx::query_scalar(
        "select o.name from organizations o \
         join organization_members m on m.org_id = o.id \
         where m.user_id = $1 and m.role = 'owner' \
         and not exists (select 1 from organization_members m2 where m2.org_id = o.id and m2.role = 'owner' and m2.user_id != $1) \
         order by o.name",
    )
    .bind(user.id)
    .fetch_all(&mut *tx)
    .await?;

    if !sole_owned_orgs.is_empty() {
        die!(
            CONFLICT,
            "Transfer ownership or delete these organizations first: {}",
            sole_owned_orgs.join(", ")
        );
    }

    let repo_ids: Vec<Uuid> = sqlx::query_scalar("select id from repositories where owner_user = $1")
        .bind(user.id)
        .fetch_all(&mut *tx)
        .await?;

    let reassigned_issues: Vec<IssueCache> = sqlx::query_as(
        "update issue_cache set \
         author_id = case when author_id = $1 then $2 else author_id end, \
         assignees = array_remove(assignees, $1) \
         where (author_id = $1 or $1 = any(assignees)) and repo_id <> all($3) \
         returning *",
    )
    .bind(user.id)
    .bind(DELETED_USER)
    .bind(&repo_ids)
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query("update issue_comment_cache set author_id = $2 where author_id = $1")
        .bind(user.id)
        .bind(DELETED_USER)
        .execute(&mut *tx)
        .await?;

    let mut cleanup = RepoCleanup::prepare(repo_ids, &mut tx).await?;
    cleanup.add_s3_key(user.avatar_s3_key().to_string());

    sqlx::query("delete from events where actor_id = $1 and (class = 'activity' or subject_id_user = $1)")
        .bind(user.id)
        .execute(&mut *tx)
        .await?;

    Event::new(
        "user.deleted",
        actor_id,
        request,
        Subject::User(user.id),
        Some(json!({
            "id": user.id,
            "username": user.username,
        })),
    )
    .save(&mut tx)
    .await?;

    let email = Email::find_primary_email(user.id, &mut tx).await?;

    sqlx::query("delete from users where id = $1").bind(user.id).execute(&mut *tx).await?;

    let base_dir: String = get_setting("repositories.base_dir", &mut tx).await?;

    cleanup.move_to_trash(format!("{base_dir}/{}", user.username), user.id, &mut tx).await?;
    cleanup.commit(tx, meili_client).await?;

    if let Some(client) = meili_client {
        if let Err(err) = client.index(USERS_MEILI_INDEX).delete_document(user.id).await {
            error!(?err, "failed to delete user from meilisearch");
        }

        if !reassigned_issues.is_empty()
            && let Err(err) = client.index(ISSUES_MEILI_INDEX).add_documents(&reassigned_issues, Some("id")).await
        {
            error!(?err, "failed to re-index reassigned issues in meilisearch");
        }
    }

    if let Some(email) = email
        && let Err(err) = send_delete_mail(user, actor_id != user.id, email.email, db_pool, queue).await
    {
        error!(?err, "failed to send account deleted email");
    }

    Ok(())
}

async fn send_delete_mail(user: &User, by_admin: bool, address: String, db_pool: &Pool, queue: &AsyncQueue) -> Result<()> {
    let mut tx = db_pool.begin().await?;

    AccountDeletedTemplate {
        username: &user.username,
        by_admin,
    }
    .send((user.username.clone(), address), &mut tx, queue)
    .await?;

    tx.commit().await?;

    Ok(())
}
