use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;
use sqlx::types::Json;
use uuid::Uuid;

pub(crate) mod oidc;
pub(crate) mod provider;

#[derive(FromRow, Debug)]
pub(crate) struct SsoConnection {
    pub(crate) user_id: Uuid,
    pub(crate) provider: String,
    pub(crate) provider_id: String,
}

#[derive(FromRow, derive_more::Debug)]
pub(crate) struct SsoState {
    #[debug(skip)]
    csrf_token: String,
    #[debug(skip)]
    state: Json<Value>,
    created_at: DateTime<Utc>,
}
