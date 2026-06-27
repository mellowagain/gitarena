use actix_web::web::ServiceConfig;
use serde::Deserialize;

pub(crate) mod api;
mod blobs;
mod git;

pub(crate) fn init(config: &mut ServiceConfig) {
    api::init(config);
    blobs::init(config);
    git::init(config); // Git smart protocol v2 routes
}

#[derive(Deserialize)]
pub(crate) struct GitRequest {
    pub(crate) namespace: String,
    pub(crate) repository: String,
}
