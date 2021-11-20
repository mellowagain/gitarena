use std::fmt::Display;

use derive_more::Display;
use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Type, Display, Debug, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
#[sqlx(rename = "repo_visibility", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub(crate) enum RepoVisibility {
    Public,
    Internal,
    Private
}
