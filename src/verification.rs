use crate::user::User;
use crate::{CONFIG, crypto, templates};

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

    user.send_template(&templates::VERIFY_EMAIL, Some([
        ("username".to_owned(), user.username.to_owned()),
        ("link".to_owned(), url)
    ].iter().cloned().collect())).await.context("Failed to send verification email.")?;

    Ok(())
}
