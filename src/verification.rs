use crate::config::get_setting;
use crate::templates::plain::render;
use crate::user::User;
use crate::{crypto, mail, template_context, templates};

use anyhow::{Context, Result};
use sqlx::{Pool, Postgres};

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
