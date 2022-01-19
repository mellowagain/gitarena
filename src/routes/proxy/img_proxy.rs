use crate::{die, err};
use crate::prelude::{AwcExtensions, HttpRequestExtensions};

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use awc::Client;
use awc::http::header::{CACHE_CONTROL, IF_MODIFIED_SINCE, IF_NONE_MATCH};
use gitarena_macros::route;
use log::debug;
use serde::Deserialize;

const PASSTHROUGH_HEADERS: [&str; 6] = [
    "cache-control",
    "content-encoding",
    "etag",
    "expires",
    "last-modified",
    "transfer-encoding"
];

// Source: https://github.com/atmos/camo/blob/master/mime-types.json
const ACCEPTED_MIME_TYPES: [&str; 43] = [
    "image/bmp",
    "image/cgm",
    "image/g3fax",
    "image/gif",
    "image/ief",
    "image/jp2",
    "image/jpeg",
    "image/jpg",
    "image/pict",
    "image/png",
    "image/prs.btif",
    "image/svg+xml",
    "image/tiff",
    "image/vnd.adobe.photoshop",
    "image/vnd.djvu",
    "image/vnd.dwg",
    "image/vnd.dxf",
    "image/vnd.fastbidsheet",
    "image/vnd.fpx",
    "image/vnd.fst",
    "image/vnd.fujixerox.edmics-mmr",
    "image/vnd.fujixerox.edmics-rlc",
    "image/vnd.microsoft.icon",
    "image/vnd.ms-modi",
    "image/vnd.net-fpx",
    "image/vnd.wap.wbmp",
    "image/vnd.xiff",
    "image/webp",
    "image/x-cmu-raster",
    "image/x-cmx",
    "image/x-icon",
    "image/x-macpaint",
    "image/x-pcx",
    "image/x-pict",
    "image/x-portable-anymap",
    "image/x-portable-bitmap",
    "image/x-portable-graymap",
    "image/x-portable-pixmap",
    "image/x-quicktime",
    "image/x-rgb",
    "image/x-xbitmap",
    "image/x-xpixmap",
    "image/x-xwindowdump"
];

#[route("/api/proxy/{url}", method = "GET", err = "text")]
pub(crate) async fn proxy(uri: web::Path<ProxyRequest>, request: HttpRequest) -> Result<impl Responder> {
    let url = &uri.url;

    if url.is_empty() {
        die!(NOT_FOUND, "Invalid url");
    }

    let bytes = hex::decode(url)?;
    let url = String::from_utf8(bytes)?;

    let mut client = Client::gitarena().get(&url);

    if let Some(header_value) = request.get_header("if-modified-since") {
        client = client.append_header((IF_MODIFIED_SINCE, header_value));
    }

    if let Some(header_value) = request.get_header("if-none-match") {
        client = client.append_header((IF_NONE_MATCH, header_value));
    }

    if let Some(header_value) = request.get_header("cache-control") {
        client = client.append_header((CACHE_CONTROL, header_value));
    }

    debug!("Image proxy request for {}", &url);

    let gateway_response = client.send().await.map_err(|err| err!(BAD_GATEWAY, "Failed to send request to gateway: {}", err))?;
    let mut response = HttpResponse::build(gateway_response.status());

    /*if length > 5242880 {
        die!(BAD_GATEWAY, "Content too big");
    }*/

    for (name, value) in gateway_response.headers() {
        let lowered_name = name.as_str().to_lowercase();
        let value_str = value.to_str()?;

        if PASSTHROUGH_HEADERS.contains(&lowered_name.as_str()) {
            response.append_header((name.as_str(), value_str));
        }

        if lowered_name == "content-type" && !ACCEPTED_MIME_TYPES.contains(&value_str) {
            die!(BAD_GATEWAY, "Response was not an image");
        }
    }

    Ok(response.streaming(gateway_response))
}

#[derive(Deserialize)]
pub(crate) struct ProxyRequest {
    pub(crate) url: String // Hex Digest
}
