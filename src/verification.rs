use crate::templates::plain::render;
use crate::user::User;
use crate::{CONFIG, crypto, mail, template_context, templates};

use std::borrow::Borrow;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use sqlx::{Postgres, Transaction};

pub(crate) async fn send_verification_mail(user: &User, transaction: &mut Transaction<'_, Postgres>) -> Result<()> {
    if !user.is_saved() {
        bail!("Unsaved user passed");
    }

    let hash = crypto::random_hex_string(32);

    sqlx::query("insert into user_verifications (user_id, hash, expires) values ($1, $2, now() + interval '1 day')")
        .bind(&user.id)
        .bind(&hash)
        .execute(transaction)
        .await?;

    let domain: &str = CONFIG.domain.borrow();
    let url = format!("{}/api/verify/{}", domain, hash);

    let template = &templates::VERIFY_EMAIL;
    let body = &template.0;
    let tags = &template.1;

    let subject = tags.get("subject").context("Template does not contain subject")?;
    let email_body = render(body.to_string(), template_context!([
        ("username".to_owned(), user.username.to_owned()),
        ("link".to_owned(), url)
    ]));

    mail::send_user_mail(user, subject, email_body).await?;

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
pub(crate) async fn is_pending(user: &User, transaction: &mut Transaction<'_, Postgres>) -> Result<bool> {
    if is_grace_period(&user).await {
        return Ok(false);
    }

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from user_verifications where user_id = $1)")
        .bind(&user.id)
        .fetch_one(transaction)
        .await?;

    Ok(exists)
}
