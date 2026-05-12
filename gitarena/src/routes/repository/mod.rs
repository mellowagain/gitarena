use actix_web::web::ServiceConfig;
use serde::Deserialize;

pub(crate) mod api;
mod archive;
mod blobs;
mod git;

pub(crate) fn init(config: &mut ServiceConfig) {
    api::init(config);
    blobs::init(config);
    git::init(config); // Git smart protocol v2 routes

    config.service(archive::tar_gz_file);
    config.service(archive::zip_file);
}

#[derive(Deserialize)]
pub(crate) struct GitRequest {
    pub(crate) namespace: String,
    pub(crate) repository: String,
}

#[derive(Deserialize)]
pub(crate) struct GitTreeRequest {
    pub(crate) namespace: String,
    pub(crate) repository: String,
    pub(crate) tree: String,
}
