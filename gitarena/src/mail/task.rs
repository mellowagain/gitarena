use crate::mail::TRANSPORTER;
use async_trait::async_trait;
use fang::{AsyncQueueable, AsyncRunnable, FangError, typetag};
use itertools::Itertools;
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::{Address, AsyncTransport, Message};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub(crate) static MAIL_TASK_TYPE: &str = "mail";

#[derive(Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub(crate) struct MailTask {
    pub(crate) from: (String, String),
    pub(crate) to: (String, String),

    pub(crate) subject: String,

    pub(crate) html: String,
    pub(crate) text: String,
}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for MailTask {
    async fn run(&self, _: &dyn AsyncQueueable) -> Result<(), FangError> {
        if let Some(transporter) = TRANSPORTER.get() {
            let from_address = Address::from_str(&self.from.1).map_err(|err| FangError { description: err.to_string() })?;
            let from_mailbox = Mailbox::new(Some(self.from.0.to_string()), from_address);

            let to_address = Address::from_str(&self.to.1).map_err(|err| FangError { description: err.to_string() })?;
            let to_mailbox = Mailbox::new(Some(self.to.0.to_string()), to_address);

            let message = Message::builder()
                .from(from_mailbox)
                .to(to_mailbox)
                .subject(&self.subject)
                .multipart(
                    MultiPart::alternative()
                        .singlepart(SinglePart::html(self.html.to_string()))
                        .singlepart(SinglePart::plain(self.text.to_string())),
                )
                .map_err(|err| FangError { description: err.to_string() })?;

            match transporter.send(message).await {
                Ok(response) if response.is_positive() => Ok(()),
                Ok(response) => Err(FangError {
                    description: response.message().join("\n"),
                }),
                Err(err) => Err(FangError { description: err.to_string() }),
            }
        } else {
            Ok(())
        }
    }

    fn task_type(&self) -> String {
        MAIL_TASK_TYPE.to_string()
    }
}
