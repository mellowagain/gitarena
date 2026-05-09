use crate::config::get_setting;
use crate::user::User;
use crate::{crypto, mail};

use anyhow::{Context, Result};
use gitarena_common::database::Pool;
use tracing::instrument;
use tracing_unwrap::OptionExt;

#[instrument(err, skip(_db_pool))]
pub(crate) async fn send_verification_mail(_user: &User, _db_pool: &Pool) -> Result<()> {
    /*assert!(user.id >= 0);

    let hash = crypto::random_hex_string(32)?;
    let mut transaction = db_pool.begin().await?;

    sqlx::query("insert into user_verifications (user_id, hash, expires) values ($1, $2, now() + interval '1 day')")
        .bind(user.id)
        .bind(&hash)
        .execute(&mut *transaction)
        .await?;

    let domain: String = get_setting("domain", &mut transaction).await?;
    let url = format!("{domain}/api/verify/{hash}");

    let template = &templates::VERIFY_EMAIL.get().unwrap_or_log();
    let body = &template.0;
    let tags = &template.1;

    let subject = tags.get("subject").context("Template does not contain subject")?;
    let email_body = render(
        body.clone(),
        template_context!([("username".to_owned(), user.username.clone()), ("link".to_owned(), url)]),
    );

    mail::send_user_mail(user, subject, email_body, db_pool).await?;

    transaction.commit().await?;

    Ok(())*/
    todo!()
}
