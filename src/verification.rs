use crate::config::get_setting;
use crate::templates::plain::render;
use crate::user::User;
use crate::{crypto, mail, template_context, templates};

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{Executor, Pool, Postgres};

pub(crate) async fn send_verification_mail(user: &User, db_pool: &Pool<Postgres>) -> Result<()> {
    assert!(user.id >= 0);

    let hash = crypto::random_hex_string(32);
    let mut transaction = db_pool.begin().await?;

    sqlx::query("insert into user_verifications (user_id, hash, expires) values ($1, $2, now() + interval '1 day')")
        .bind(&user.id)
        .bind(&hash)
        .execute(&mut transaction)
        .await?;

    let domain = get_setting::<String, _>("domain", &mut transaction).await?;
    let url = format!("{}/api/verify/{}", domain, hash);

    let template = &templates::VERIFY_EMAIL;
    let body = &template.0;
    let tags = &template.1;

    let subject = tags.get("subject").context("Template does not contain subject")?;
    let email_body = render(body.to_string(), template_context!([
        ("username".to_owned(), user.username.to_owned()),
        ("link".to_owned(), url)
    ]));

    mail::send_user_mail(user, subject, email_body, db_pool).await?;

    transaction.commit().await?;

    Ok(())
}

/// Checks if the user is still in grace period (24 hours).
/// In this grace period the user can do all actions even without having a verified email address
pub(crate) async fn is_grace_period(user: &User) -> bool {
    let user_creation = &user.created_at;
    let now = Utc::now();
    let difference = user_creation.signed_duration_since(now);

    difference.num_hours() < 24
}

/// Checks if the user has failed to verify their email address within 24 hours
pub(crate) async fn is_pending<'e, E: Executor<'e, Database = Postgres>>(user: &User, executor: E) -> Result<bool> {
    if is_grace_period(&user).await {
        return Ok(false);
    }

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from user_verifications where user_id = $1)")
        .bind(&user.id)
        .fetch_one(executor)
        .await?;

    Ok(exists)
}
