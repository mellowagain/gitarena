use crate::database::Pool;
use crate::mail::templates::{EmailTemplate, VerifyEmailTemplate};
use crate::prelude::MapToFangError;
use crate::user::User;
use crate::{TASK_DB_POOL, crypto};
use anyhow::Result;
use async_trait::async_trait;
use fang::{AsyncQueue, AsyncQueueable, AsyncRunnable, Deserialize, FangError, Scheduled, Serialize, typetag};
use gitarena_macros::from_config;
use tracing::{info, instrument};
use uuid::Uuid;

#[instrument(err, skip(queue, db_pool))]
pub(crate) async fn send_verification_mail(user: &User, email: String, queue: &AsyncQueue, db_pool: &Pool) -> Result<()> {
    let (smtp_enabled, domain) = from_config!(
        "smtp.enabled" => bool,
        "domain.app" => String,
    );

    if !smtp_enabled {
        return Ok(());
    }

    let hash = crypto::random_hex_string(32)?;
    let mut tx = db_pool.begin().await?;

    sqlx::query("insert into user_verifications (id, user_id, email, hash, expires) values ($1, $2, $3, $4, now() + interval '1 day')")
        .bind(Uuid::now_v7())
        .bind(user.id)
        .bind(&email)
        .bind(&hash)
        .execute(&mut *tx)
        .await?;

    VerifyEmailTemplate {
        link: &format!("{domain}/api/verify/{hash}"),
    }
    .send((user.username.clone(), email), &mut tx, queue)
    .await?;

    tx.commit().await?;

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
