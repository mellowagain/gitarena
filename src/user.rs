use crate::templates::plain::{render, Template, TemplateContext};
use crate::{CONFIG, crypto, mail};

use std::borrow::Borrow;

use anyhow::{bail, Context, Result};
use lettre::Message;
use sqlx::{Executor, FromRow, PgPool, Postgres, Transaction};

#[derive(FromRow)]
pub(crate) struct User {
    pub(crate) id: i32,
    pub(crate) username: String, // Ascii-only
    pub(crate) email: String,
    pub(crate) password: String,
    pub(crate) salt: String,
    pub(crate) email_verified: bool
}

impl User {
    /// Creates a new user object. The user is not yet saved to the database.
    pub(crate) fn new(username: String, email: String, raw_password: String) -> Result<Self> {
        let (password, salt) = crypto::hash_password(raw_password).context("Failed to hash password.")?;

        Ok(User {
            id: -1, // User has not yet been placed in the database.
            username,
            email,
            password,
            salt,
            email_verified: false
        })
    }

    pub(crate) fn is_saved(&self) -> bool {
        self.id > -1
    }

    /// Saves this user to the database and populates the user id.
    /// If a error gets returned, the transaction should be cancelled in order to not insert the user
    pub(crate) async fn save(&mut self, transaction: &mut Transaction<'_, Postgres>) -> Result<()> {
        if self.is_saved() {
            bail!("User id ({}) already saved to database", self.id);
        }

        transaction.execute(
            sqlx::query("insert into users (username, email, password, salt) values ($1, $2, $3, $4);")
                .bind(&self.username)
                .bind(&self.email)
                .bind(&self.password)
                .bind(&self.salt)
        ).await.context("Failed to insert user into database.")?;

        let (id,): (i64,) = sqlx::query_as("select currval(pg_get_serial_sequence('users', 'id'));")
            .fetch_one(transaction)
            .await
            .context("Failed to acquire user id.")?;

        if id > i32::MAX as i64 {
            bail!("Returned user id ({}) does not fit into i32.", id);
        }

        self.id = id as i32;

        Ok(())
    }

    pub(crate) async fn send_mail(&self, subject: &String, body: String) -> Result<()> {
        let address: &str = CONFIG.smtp.email_address.borrow();

        let message = Message::builder()
            .from(format!("GitArena <{}>", address).parse().context("Unable to parse `from` email.")?)
            .to(format!("{} <{}>", self.username, self.email).parse().context("Unable to parse `to` email.")?)
            .subject(subject)
            .body(body)
            .context("Unable to build email.")?;

        Ok(mail::send_mail(message).await?)
    }

    pub(crate) async fn send_template(&self, template: &Template, context: Option<TemplateContext>) -> Result<()> {
        let (body, tags) = template;
        let email_body = render(body.to_string(), context);

        if !tags.contains_key("subject") {
            bail!("Template {} does not contain subject tag.", tags.get("template_name").unwrap());
        }

        let subject = tags.get("subject").unwrap();

        Ok(self.send_mail(subject, email_body).await?)
    }
}
