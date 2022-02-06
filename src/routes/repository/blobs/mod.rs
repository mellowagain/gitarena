use actix_web::web::ServiceConfig;
use serde::Deserialize;

mod blob;

pub(crate) fn init(config: &mut ServiceConfig) {
    config.service(blob::view_blob);
    config.service(blob::view_raw_blob);
}

#[derive(Deserialize)]
pub(crate) struct BlobRequest {
    // Currently not implemented in actix-web: https://github.com/actix/actix-web/issues/2626
    //#[serde(flatten)]
    //pub(crate) repo: GitRequest,
    pub(crate) username: String,
    pub(crate) repository: String,

    pub(crate) tree: String,
    pub(crate) blob: String
}
