use crate::config::get_setting;
use crate::database::Database;
use crate::mail::task::MailTask;
use anyhow::{Context, Result};
use askama::Template;
use fang::{AsyncQueue, AsyncQueueable, AsyncRunnable};
use sqlx::Transaction;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use tracing::instrument;

#[derive(Template, Debug)]
#[template(path = "verify_email.txt", escape = "none")]
pub(crate) struct VerifyEmailTemplate<'a> {
    pub(crate) link: &'a str,
}

impl EmailTemplate for VerifyEmailTemplate<'_> {
    fn subject(&self) -> String {
        "Verify your email address".to_string()
    }
}

#[derive(Template, Debug)]
#[template(path = "org_invite.txt", escape = "none")]
pub(crate) struct OrgInviteTemplate<'a> {
    pub(crate) inviter: &'a str,
    pub(crate) org: &'a str,
    pub(crate) role: &'a str,
    pub(crate) link: &'a str,
}

impl EmailTemplate for OrgInviteTemplate<'_> {
    fn subject(&self) -> String {
        format!("You've been invited to join {}", self.org)
    }
}

#[derive(Template, Debug)]
#[template(path = "new_login.txt", escape = "none")]
pub(crate) struct NewLoginTemplate<'a> {
    pub(crate) time: &'a str,
    pub(crate) location: &'a str,
    pub(crate) device: &'a str,
    pub(crate) method: &'a str,
}

impl EmailTemplate for NewLoginTemplate<'_> {
    fn subject(&self) -> String {
        "New sign-in detected".to_string()
    }
}

pub(crate) trait EmailTemplate: Template + Debug {
    fn subject(&self) -> String;

    #[instrument(skip(tx))]
    async fn send(&self, to: (String, String), tx: &mut Transaction<'_, Database>, queue: &AsyncQueue) -> Result<()> {
        if !get_setting::<bool>("smtp.enabled", tx).await? {
            return Ok(());
        }

        let address: String = get_setting("smtp.address", tx).await?;
        let domain: String = get_setting("domain.app", tx).await?;

        let mut values: HashMap<_, Box<dyn Any>> = HashMap::with_capacity(2);
        values.insert("instance_name", Box::new("GitArena".to_string()));
        values.insert("domain", Box::new(domain));

        let task = MailTask {
            from: ("GitArena".to_string(), address),
            to,
            subject: self.subject(),
            body: self.render_with_values(&values).context("failed to render email template")?,
        };

        queue.insert_task(&task as &dyn AsyncRunnable).await.context("failed to enqueue mail task")?;
        Ok(())
    }
}
