use derive_more::Display;
use serde::{Deserialize, Serialize};
use sqlx::Type;
use utoipa::ToSchema;

#[derive(Type, Display, Debug, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
#[sqlx(type_name = "repo_visibility", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
#[schema(rename_all = "lowercase")]
pub(crate) enum RepoVisibility {
    Public,
    Internal,
    Private,
}
