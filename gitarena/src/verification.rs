use crate::crypto;
use crate::mail::task::MailTask;
use crate::mail::templates::VerifyEmailTemplate;
use crate::user::User;
use anyhow::{Context, Result};
use askama::Template;
use fang::{AsyncQueue, AsyncQueueable, AsyncRunnable};
use gitarena_common::database::Pool;
use gitarena_macros::from_config;
use tracing::instrument;

#[instrument(err, skip(db_pool))]
pub(crate) async fn send_verification_mail(user: &User, email: String, queue: &AsyncQueue, db_pool: &Pool) -> Result<()> {
    assert!(user.id >= 0);

    let (domain, smtp_address) = from_config!(
        "domain" => String,
        "smtp.address" => String,
    );

    let hash = crypto::random_hex_string(32)?;
    let mut transaction = db_pool.begin().await?;

    sqlx::query("insert into user_verifications (user_id, hash, expires) values ($1, $2, now() + interval '1 day')")
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

    queue.insert_task(&task as &dyn AsyncRunnable).await.context("failed to enqueue mail task")?;

    transaction.commit().await?;

    Ok(())
}
