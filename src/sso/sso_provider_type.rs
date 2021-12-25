use crate::sso::github_sso::GitHubSSO;
use crate::sso::sso_provider::SSOProvider;

use std::result::Result as StdResult;
use std::str::FromStr;

use derive_more::Display;
use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Type, Display, Debug, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
#[sqlx(rename = "sso_provider", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub(crate) enum SSOProviderType {
    GitHub
}

impl SSOProviderType {
    pub(crate) fn get_implementation(&self) -> impl SSOProvider {
        match self {
            SSOProviderType::GitHub => GitHubSSO
        }
    }
}

impl FromStr for SSOProviderType {
    type Err = ();

    fn from_str(input: &str) -> StdResult<Self, Self::Err> {
        let lower_input = input.to_lowercase();

        match lower_input.as_str() {
            "github" => Ok(SSOProviderType::GitHub),
            _ => Err(())
        }
    }
}
