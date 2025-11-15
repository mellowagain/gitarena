use anyhow::Result;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

#[async_trait(?Send)]
pub(crate) trait OAuthRequest<T: DeserializeOwned = SerdeMap> {
    async fn request_data(endpoint: &'static str, token: &str) -> Result<T>;
}

pub(crate) type SerdeMap = Map<String, Value>;
