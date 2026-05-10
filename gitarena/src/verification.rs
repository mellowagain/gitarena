use crate::config::get_setting;
use crate::mail::task::MailTask;
use crate::mail::templates::VerifyEmailTemplate;
use crate::prelude::MapToFangError;
use crate::user::User;
use crate::{TASK_DB_POOL, crypto};
use anyhow::{Context, Result};
use askama::Template;
use async_trait::async_trait;
use fang::{AsyncQueue, AsyncQueueable, AsyncRunnable, Deserialize, FangError, Scheduled, Serialize, typetag};
use gitarena_common::database::Pool;
use gitarena_macros::from_config;
use tracing::{info, instrument};
use uuid::Uuid;

#[instrument(err, skip(queue, db_pool))]
pub(crate) async fn send_verification_mail(user: &User, email: String, queue: &AsyncQueue, db_pool: &Pool) -> Result<()> {
    let (smtp_enabled, domain) = from_config!(
        "smtp.enabled" => bool,
        "domain" => String,
    );

    if !smtp_enabled {
        return Ok(());
    }

    let hash = crypto::random_hex_string(32)?;
    let mut transaction = db_pool.begin().await?;

    let smtp_address = get_setting("smtp.address", &mut transaction).await?;

    sqlx::query("insert into user_verifications (id, user_id, hash, expires) values ($1, $2, $3, now() + interval '1 day')")
        .bind(Uuid::now_v7())
        .bind(user.id)
        .bind(&hash)
        .execute(&mut *transaction)
        .await?;

    let template = VerifyEmailTemplate {
        link: &format!("{domain}/api/verify/{hash}"),
        instance_name: "GitArena",
        domain: &domain,
    };

    let task = MailTask {
        from: ("GitArena".to_string(), smtp_address),
        to: (user.username.clone(), email),
        subject: template.subject(),
        body: template.render().context("failed to render verify email template")?,
    };

    transaction.commit().await?;

    queue.insert_task(&task as &dyn AsyncRunnable).await.context("failed to enqueue mail task")?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub(crate) struct ExpiredVerifyLinkRemovalTask {}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for ExpiredVerifyLinkRemovalTask {
    #[instrument(skip(_client))]
    async fn run(&self, _client: &dyn AsyncQueueable) -> Result<(), FangError> {
        let db_pool = TASK_DB_POOL.get().ok_or_else(|| FangError {
            description: "task db pool OnceCell is empty".to_string(),
        })?;

        let mut tx = db_pool.begin().await.fang()?;

        let result = sqlx::query("delete from user_verifications where expires < now()")
            .execute(&mut *tx)
            .await
            .fang()?;

        tx.commit().await.fang()?;

        info!(count = %result.rows_affected(), "deleted expired user verification links");
        Ok(())
    }

    fn uniq(&self) -> bool {
        true
    }

    fn cron(&self) -> Option<Scheduled> {
        // daily at 3am
        Some(Scheduled::CronPattern("0 0 3 * * * *".to_string()))
    }
}
