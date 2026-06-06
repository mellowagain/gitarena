use anyhow::Result;
use bytes::Bytes;
use futures::stream::BoxStream;
use object_store::path::Path;
use object_store::{ObjectStore, ObjectStoreExt, PutPayload};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReleaseAssets {
    pub(crate) id: Uuid,
    pub(crate) release_id: Uuid,

    pub(crate) name: String,
    pub(crate) size: i64,
    pub(crate) hash: String,

    pub(crate) available: bool,
    pub(crate) downloads: i64,

    pub(crate) os: Option<Os>,
    pub(crate) arch: Option<Arch>,
    pub(crate) libc: Option<Libc>,
    pub(crate) kind: Option<Kind>,
}

impl ReleaseAssets {
    pub(crate) async fn get(&self, store: &dyn ObjectStore) -> Result<BoxStream<'static, object_store::Result<Bytes>>> {
        Ok(store.get(&self.s3_key()).await?.into_stream())
    }

    pub(crate) async fn put(&self, store: &dyn ObjectStore, payload: PutPayload) -> Result<()> {
        store.put(&self.s3_key(), payload).await?;
        Ok(())
    }

    pub(crate) fn s3_key(&self) -> Path {
        Path::from(format!("releases/{}/{}/{}", self.release_id, self.id, self.name))
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "asset_os", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub(crate) enum Os {
    Linux,
    Windows,
    MacOS,
    FreeBSD,
    OpenBSD,
    NetBSD,
    Android,
    IOS,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "asset_arch", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub(crate) enum Arch {
    X86_64,
    I686,
    Aarch64,
    ArmV7,
    ArmV6,
    RiscV64,
    Loongarch64,
    PowerPc64,
    S390x,
    Wasm32,
    Universal,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "asset_libc", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub(crate) enum Libc {
    Gnu,
    Musl,
    Msvc,
    MingW,
    Bionic,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "asset_kind", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    Binary,
    Installer,
    Library,
    Source,
    SBOM,
    Other,
}
