use crate::crypto;

use anyhow::Result;
use chrono::{DateTime, Utc, NaiveDateTime, NaiveDate};
use lazy_static::lazy_static;
use serde::Serialize;
use sqlx::FromRow;

#[derive(FromRow, Serialize)]
pub(crate) struct User {
    pub(crate) id: i32,
    pub(crate) username: String,
    pub(crate) email: String,
    pub(crate) password: String,
    pub(crate) disabled: bool,
    pub(crate) session: String,
    pub(crate) created_at: DateTime<Utc>
}

impl User {
    pub(crate) fn new() -> User {
        lazy_static! {
            static ref EPOCH: NaiveDateTime = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
            static ref UTC_DT: DateTime<Utc> = DateTime::<Utc>::from_utc(*EPOCH, Utc);
        }

        User {
            id: -1,
            username: "".to_owned(),
            email: "".to_owned(),
            password: "".to_owned(),
            disabled: false,
            session: "".to_owned(),
            created_at: *UTC_DT
        }
    }

    pub(crate) fn username(&mut self, username: String) -> &mut User {
        self.username = username;
        self
    }

    pub(crate) fn email(&mut self, email: String) -> &mut User {
        self.email = email;
        self
    }

    pub(crate) fn password(&mut self, password: String) -> &mut User {
        self.password = password;
        self
    }

    pub(crate) fn raw_password(&mut self, raw_password: &String) -> Result<&mut User> {
        self.password = crypto::hash_password(&raw_password)?;
        Ok(self)
    }

    pub(crate) fn disabled(&mut self, disabled: bool) -> &mut User {
        self.disabled = disabled;
        self
    }

    pub(crate) fn identity_str(&self) -> String {
        format!("{}${}", &self.id, &self.session)
    }

    pub(crate) fn is_valid(&self) -> bool {
        !self.username.is_empty() &&
        !self.email.is_empty() &&
        !self.password.is_empty()
    }

    pub(crate) fn is_saved(&self) -> bool {
        self.id > -1
    }
}
