use crate::user::User;

use anyhow::{Context, Result};
use gitarena_macros::from_config;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, Message, Tokio02Connector, Tokio02Transport};
use sqlx::{Pool, Postgres};

pub(crate) async fn send_user_mail(user: &User, subject: &String, body: String, db_pool: &Pool<Postgres>) -> Result<()> {
    let address: String = from_config!("smtp.address" => String);

    let message = Message::builder()
        .from(format!("GitArena <{}>", address).parse().context("Unable to parse `from` email.")?)
        .to(format!("{} <{}>", user.username, user.email).parse().context("Unable to parse `to` email.")?)
        .subject(subject)
        .body(body)
        .context("Unable to build email.")?;

    Ok(send_mail(message, db_pool).await?)
}

pub(crate) async fn send_mail(message: Message, db_pool: &Pool<Postgres>) -> Result<()> {
    let (server, username, password, port, tls): (String, String, String, i32, bool) = from_config!(
        "smtp.server" => String,
        "smtp.username" => String,
        "smtp.password" => String,
        "smtp.port" => i32,
        "smtp.tls" => bool
    );

    let credentials = Credentials::new(username, password);
    let transporter;

    if tls {
        transporter = AsyncSmtpTransport::<Tokio02Connector>::relay(server.as_str())
            .context("Unable to create TLS connection")?
            .port(port as u16)
            .credentials(credentials)
            .build();
    } else {
        transporter = AsyncSmtpTransport::<Tokio02Connector>::builder_dangerous(server.as_str())
            .port(port as u16)
            .credentials(credentials)
            .build();
    }

    transporter.send(message).await.context("Unable to send email")?;

    Ok(())
}
