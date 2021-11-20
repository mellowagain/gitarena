use std::fmt::Display;

use derive_more::Display;
use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Type, Display, Debug, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
#[sqlx(rename = "access_level", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub(crate) enum AccessLevel {
    Viewer,
    Supporter,
    Coder,
    Manager,
    Admin
}

// Currently all these methods are hard coded but in the future they will be configurable on a per repo/org basis
impl AccessLevel {
    pub(crate) fn can_view(&self) -> bool {
        true
    }

    pub(crate) fn can_manage_issues(&self) -> bool {
        match self {
            AccessLevel::Viewer | AccessLevel::Coder => false,
            AccessLevel::Supporter | AccessLevel::Manager | AccessLevel::Admin => true
        }
    }

    pub(crate) fn can_push(&self) -> bool {
        match self {
            AccessLevel::Viewer | AccessLevel::Supporter => false,
            AccessLevel::Coder | AccessLevel::Manager | AccessLevel::Admin => true
        }
    }

    pub(crate) fn can_admin(&self) -> bool {
        matches!(self, AccessLevel::Admin)
    }
}
