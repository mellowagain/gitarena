use chrono::{DateTime, Utc};
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
    pub(crate) fn identity_str(&self) -> String {
        format!("{}${}", &self.id, &self.session)
    }

    // TODO: Remove this as it's only used by verification check which will be refactored sooner or later
    // Maybe with a call to `assert!(id > -1)`
    pub(crate) fn is_saved(&self) -> bool {
        self.id > -1
    }
}
