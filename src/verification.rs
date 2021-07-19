use crate::templates::plain::render;
use crate::user::User;
use crate::{CONFIG, crypto, mail, templates, template_context};

use std::borrow::Borrow;

use anyhow::bail;
use anyhow::{Context, Result};
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

/// Checks if the user has failed to verify their email address within 24 hours
pub(crate) async fn has_failed(_user: &User, _transaction: &mut Transaction<'_, Postgres>) -> Result<bool> {
    todo!()
}
