use crate::database::Pool;
use anyhow::Result;
use gitarena_macros::{from_config, from_optional_config};
use object_store::aws::AmazonS3;
use object_store::aws::AmazonS3Builder;
use tracing::info;

pub(crate) type Storage = Option<AmazonS3>;

pub(crate) async fn init(db_pool: &Pool) -> Result<Storage> {
    let s3_enabled = from_config!("s3.enabled" => bool);

    if !s3_enabled {
        info!("object storage unavailable as s3 is disabled");
        return Ok(None);
    }

    let (bucket, region, access_key_id, secret_access_key, force_path_style) = from_config!(
        "s3.bucket" => String,
        "s3.region" => String,
        "s3.access_key_id" => String,
        "s3.secret_access_key" => String,
        "s3.force_path_style" => bool,
    );

    let endpoint = from_optional_config!("s3.endpoint" => String);

    let mut builder = AmazonS3Builder::new()
        .with_bucket_name(bucket)
        .with_region(region)
        .with_access_key_id(access_key_id)
        .with_secret_access_key(secret_access_key)
        .with_virtual_hosted_style_request(!force_path_style);

    if let Some(endpoint) = endpoint {
        builder = builder.with_endpoint(endpoint);
    }

    let store = builder.build()?;

    info!("object storage (s3) initialized");

    Ok(Some(store))
}
