//! The email model is similar to other software development platforms. The following applies:
//!
//! - The **primary email** is used for avatar detection using Gravatar and password resets.
//! - The **commit email** is used for web based Git actions such as merge request mergers.
//! - The **notification email** is used for account related notifications. It may be overridden on a per-group basis.
//! - The **public email** is displayed on the user profile.
//! - All emails will be used to identify Git commits and incoming emails (e.g. issue creation by email).

use crate::user::User;

use std::fmt::{Debug, Formatter, Result as FmtResult, Write};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use derive_more::Display;
use gitarena_macros::from_config;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Serialize;
use sqlx::{Executor, FromRow, Pool, Postgres};

#[derive(FromRow, Display, Serialize)]
#[display(fmt = "{}", email)]
pub(crate) struct Email {
    pub(crate) id: i32,
    pub(crate) owner: i32,
    pub(crate) email: String,

    // Flags are not mutually exclusive, meaning that a single email address may have set `true` on more than one of these fields
    // Per user only a single email address may have set `true` on one or more of the following fields
    // This means that per user there is guaranteed to be only one primary, commit, notification and public email address. Not more and not less.
    // It is also possible for an email address to not have a single of these fields set to `true`
    pub(crate) primary: bool,
    pub(crate) commit: bool,
    pub(crate) notification: bool,
    pub(crate) public: bool,

    pub(crate) created_at: DateTime<Local>,
    pub(crate) verified_at: Option<DateTime<Local>>
}

impl Email {
    pub(crate) fn as_mailbox(&self, name: Option<String>) -> Result<Mailbox> {
        Ok(Mailbox::new(name, self.email.parse()?))
    }

    // This method should only be called on the primary email upon login
    pub(crate) fn is_allowed_login(&self) -> bool {
        assert!(self.primary);

        match self.verified_at {
            Some(_) => true,
            None => self.created_at.signed_duration_since(Local::now()).num_hours() < 24
        }
    }
}

macro_rules! generate_find {
    ($method_name:ident, $field:literal) => {
        pub(crate) async fn $method_name<'e, E: Executor<'e, Database = Postgres>, U: Into<i32>>(user: U, executor: E) -> Result<Option<Email>> {
            let query = concat!("select * from emails where owner = $1 and ", $field, " = true limit 1");
            Email::find_specific_email(user, query, executor).await
        }
    }
}

impl Email {
    generate_find!(find_primary_email, "\"primary\"");
    generate_find!(find_commit_email, "commit");
    generate_find!(find_notification_email, "notification");
    generate_find!(find_public_email, "public");

    // Private helper called by the functions defined using the `generate_find!` macro
    async fn find_specific_email<'e, E, U>(user: U, query: &'static str, executor: E) -> Result<Option<Email>>
        where E: Executor<'e, Database = Postgres>,
              U: Into<i32>
    {
        let email: Option<Email> = sqlx::query_as(query)
            .bind(user.into())
            .fetch_optional(executor)
            .await?;

        Ok(email)
    }
}

impl Debug for Email {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Email(user {}: {}", self.owner, self.email)?;

        if self.primary {
            f.write_str(", primary")?;
        }

        if self.commit {
            f.write_str(", commit")?;
        }

        if self.notification {
            f.write_str(", notification")?;
        }

        if self.public {
            f.write_str(", public")?;
        }

        f.write_str(match self.verified_at {
            Some(_) => ", verified",
            None => ", NOT verified"
        })?;

        f.write_char(')')
    }
}

pub(crate) async fn get_root_email(db_pool: &Pool<Postgres>) -> Result<String> {
    let address: String = from_config!("smtp.address" => String);

    Ok(address)
}

pub(crate) async fn get_root_mailbox(db_pool: &Pool<Postgres>) -> Result<Mailbox> {
    let address = get_root_email(db_pool).await?;

    // TODO: Allow customization of display name for email address
    Ok(Mailbox::new(Some("GitArena".to_owned()), address.parse()?))
}

pub(crate) async fn send_user_mail(user: &User, subject: &str, body: String, db_pool: &Pool<Postgres>) -> Result<()> {
    // This is in an extra block so `transaction` gets dropped early
    let email = {
        let mut transaction = db_pool.begin().await?;

        // Every *valid* user has a notification email address in the database, so .unwrap is fine
        let email = Email::find_notification_email(user, &mut transaction)
            .await?
            .ok_or_else(|| anyhow!("User {} has no notification email address", user))?;

        transaction.commit().await?;

        email
    };

    let message = Message::builder()
        .from(get_root_mailbox(db_pool).await?)
        .to(email.as_mailbox(Some(user.username.to_owned()))?)
        .subject(subject)
        .body(body)
        .context("Unable to build email.")?;

    send_mail(message, db_pool).await
}

async fn send_mail(message: Message, db_pool: &Pool<Postgres>) -> Result<()> {
    let (server, username, password, port, tls): (String, String, String, i32, bool) = from_config!(
        "smtp.server" => String,
        "smtp.username" => String,
        "smtp.password" => String,
        "smtp.port" => i32,
        "smtp.tls" => bool
    );

    let credentials = Credentials::new(username, password);

    let transporter = if tls {
        AsyncSmtpTransport::<Tokio1Executor>::relay(server.as_str())
            .context("Unable to create TLS connection")?
            .port(port as u16)
            .credentials(credentials)
            .build()
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(server.as_str())
            .port(port as u16)
            .credentials(credentials)
            .build()
    };

    transporter.send(message).await.context("Unable to send email")?;

    Ok(())
}
