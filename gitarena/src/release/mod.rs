use crate::database::Database;
use crate::release::assets::ReleaseAssets;
use anyhow::Result;
use serde::Serialize;
use sqlx::{FromRow, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

pub(crate) mod assets;

#[derive(Debug, Serialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Release {
    pub(crate) id: Uuid,
    pub(crate) repo_id: Uuid,

    pub(crate) title: String,
    pub(crate) description: Option<String>,

    pub(crate) author: Uuid,

    pub(crate) tag: String,
    pub(crate) pre_release: bool,
}

impl Release {
    pub(crate) async fn assets(&self, tx: &mut Transaction<'_, Database>) -> Result<Vec<ReleaseAssets>> {
        Ok(sqlx::query_as("select * from release_assets where release_id = $1 and available = true")
            .bind(self.id)
            .fetch_all(&mut **tx)
            .await?)
    }
}
