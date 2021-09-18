use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Type, Debug, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
#[sqlx(rename="repo_visibility", rename_all="lowercase")]
#[serde(rename_all(serialize="lowercase", deserialize="lowercase"))]
pub(crate) enum RepoVisibility {
    Public,
    Internal,
    Private
}

impl Display for RepoVisibility {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoVisibility::Public => f.write_str("Public"),
            RepoVisibility::Internal => f.write_str("Internal"),
            RepoVisibility::Private => f.write_str("Private")
        }
    }
}
