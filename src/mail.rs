use crate::CONFIG;
use crate::user::User;

use std::borrow::Borrow;

use anyhow::{Context, Result};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, Message, Tokio02Connector, Tokio02Transport};

pub(crate) async fn send_user_mail(user: &User, subject: &String, body: String) -> Result<()> {
    let address: &str = CONFIG.smtp.email_address.borrow();

    let message = Message::builder()
        .from(format!("GitArena <{}>", address).parse().context("Unable to parse `from` email.")?)
        .to(format!("{} <{}>", user.username, user.email).parse().context("Unable to parse `to` email.")?)
        .subject(subject)
        .body(body)
        .context("Unable to build email.")?;

    Ok(send_mail(message).await?)
}

pub(crate) async fn send_mail(message: Message) -> Result<()> {
    let server: &str = CONFIG.smtp.server.borrow();
    let username: &str = CONFIG.smtp.username.borrow();
    let password: &str = CONFIG.smtp.password.borrow();
    let raw_port: &i64 = CONFIG.smtp.port.borrow();
    let tls: &bool = CONFIG.smtp.tls.borrow();
    let port = *raw_port as u16;

    let credentials = Credentials::new(username.to_owned(), password.to_owned());
    let transporter;

    if *tls {
        transporter = AsyncSmtpTransport::<Tokio02Connector>::relay(server)
            .context("Unable to create TLS connection.")?
            .port(port)
            .credentials(credentials)
            .build();
    } else {
        transporter = AsyncSmtpTransport::<Tokio02Connector>::builder_dangerous(server)
            .port(port)
            .credentials(credentials)
            .build();
    }

    transporter.send(message).await.context("Unable to send e-mail.")?;

    Ok(())
}
