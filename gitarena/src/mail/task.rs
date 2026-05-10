use crate::mail::TRANSPORTER;
use crate::prelude::MapToFangError;
use async_trait::async_trait;
use fang::{AsyncQueueable, AsyncRunnable, FangError, typetag};
use itertools::Itertools;
use lettre::message::Mailbox;
use lettre::{Address, AsyncTransport, Message};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{debug, error, instrument};

pub(crate) static MAIL_TASK_TYPE: &str = "mail";

#[derive(derive_more::Debug, Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub(crate) struct MailTask {
    #[debug("{} <{}>", from.0, from.1)]
    pub(crate) from: (String, String),
    #[debug("{} <{}>", to.0, to.1)]
    pub(crate) to: (String, String),

    pub(crate) subject: String,

    #[debug(skip)]
    pub(crate) body: String,
}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for MailTask {
    #[instrument(skip(_client))]
    async fn run(&self, _client: &dyn AsyncQueueable) -> Result<(), FangError> {
        if let Some(transporter) = TRANSPORTER.get() {
            let from_address = Address::from_str(&self.from.1).fang()?;
            let from_mailbox = Mailbox::new(Some(self.from.0.clone()), from_address);

            let to_address = Address::from_str(&self.to.1).fang()?;
            let to_mailbox = Mailbox::new(Some(self.to.0.clone()), to_address);

            let message = Message::builder()
                .from(from_mailbox)
                .to(to_mailbox)
                .subject(&self.subject)
                .body(self.body.clone())
                .fang()?;

            let result = match transporter.send(message).await {
                Ok(response) if response.is_positive() => Ok(()),
                Ok(response) => Err(FangError {
                    description: response.message().join("\n"),
                }),
                Err(err) => Err(FangError { description: err.to_string() }),
            };

            match &result {
                Ok(()) => debug!("email sent"),
                Err(err) => error!(?err, "error occurred during email sending"),
            }

            result
        } else {
            Ok(())
        }
    }

    fn task_type(&self) -> String {
        MAIL_TASK_TYPE.to_string()
    }
}
