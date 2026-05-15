use crate::mail::task::MAIL_TASK_TYPE;
use crate::passkey::ExpiredWebAuthnChallengesRemovalTask;
use crate::verification::ExpiredVerifyLinkRemovalTask;
use anyhow::{Context, Result};
use fang::{AsyncQueue, AsyncQueueable, AsyncRunnable, AsyncWorkerPool, SleepParams};
use std::env;
use std::time::Duration;
use tracing::info;

pub(crate) async fn init() -> Result<AsyncQueue> {
    // todo: this makes `DATABASE_URL` mandatory
    let mut queue = AsyncQueue::builder()
        .uri(env::var("DATABASE_URL").context("unable to find mandatory `DATABASE_URL` env variable")?)
        .max_pool_size(u32::try_from(num_cpus::get()).context("cpu cores too big for u32")?)
        .build();

    queue.connect().await.context("task queue failed to connect to postgres database")?;

    let mut general_worker_pool = AsyncWorkerPool::<AsyncQueue>::builder()
        .number_of_workers(u32::try_from(num_cpus::get()).context("cpu cores too big for u32")?)
        .sleep_params(
            SleepParams::builder()
                .sleep_period(Duration::from_secs(5))
                .min_sleep_period(Duration::from_secs(5))
                .max_sleep_period(Duration::from_secs(30))
                .sleep_step(Duration::from_secs(5))
                .build(),
        )
        .queue(queue.clone())
        .build();

    general_worker_pool.start().await;

    // users expect emails to arrive fast so the email worker pool has low sleep params
    let mut email_worker_pool = AsyncWorkerPool::<AsyncQueue>::builder()
        .number_of_workers(2_u32)
        .sleep_params(
            SleepParams::builder()
                .sleep_period(Duration::from_secs(1))
                .min_sleep_period(Duration::from_secs(1))
                .max_sleep_period(Duration::from_secs(5))
                .sleep_step(Duration::from_millis(500))
                .build(),
        )
        .task_type(MAIL_TASK_TYPE.to_string())
        .queue(queue.clone())
        .build();

    email_worker_pool.start().await;

    schedule_cron_jobs(&queue).await?;
    Ok(queue)
}

async fn schedule_cron_jobs(queue: &AsyncQueue) -> Result<()> {
    {
        let cron = ExpiredVerifyLinkRemovalTask {};

        let task = queue
            .schedule_task(&cron as &dyn AsyncRunnable)
            .await
            .context("failed to schedule remove expired verify link cron job")?;

        info!(id = %task.id, "scheduled cron job: remove expired verify links");
    }
    {
        let cron = ExpiredWebAuthnChallengesRemovalTask {};

        let task = queue
            .schedule_task(&cron as &dyn AsyncRunnable)
            .await
            .context("failed to schedule remove expired webauthn challenges cron job")?;

        info!(id = %task.id, "scheduled cron job: remove expired webauthn challenges");
    }
    Ok(())
}
