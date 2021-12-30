use crate::error::GAErrors::HttpError;
use crate::prelude::HttpRequestExtensions;
use crate::utils::reqwest_actix_stream::ResponseStream;

use actix_web::http::header::CONTENT_LENGTH;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::{Context, Result};
use gitarena_macros::route;
use log::debug;
use reqwest::Client;
use serde::Deserialize;

const PASSTHROUGH_HEADERS: [&'static str; 6] = [
    "cache-control",
    "content-encoding",
    "etag",
    "expires",
    "last-modified",
    "transfer-encoding"
];

// https://github.com/atmos/camo/blob/master/mime-types.json
const ACCEPTED_MIME_TYPES: [&'static str; 43] = [
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
        return Err(HttpError(404, "Invalid url".to_owned()).into());
    }

    let bytes = hex::decode(url)?;
    let url = String::from_utf8(bytes)?;

    let mut client = Client::new().get(&url);

    if let Some(header_value) = request.get_header("if-modified-since") {
        client = client.header("if-modified-since", header_value);
    }

    if let Some(header_value) = request.get_header("if-none-match") {
        client = client.header("if-none-match", header_value);
    }

    if let Some(header_value) = request.get_header("cache-control") {
        client = client.header("cache-control", header_value);
    }

    debug!("Image proxy request for {}", &url);

    let gateway_response = client.send().await.context("Failed to send request to gateway")?;
    let mut response = HttpResponse::build(gateway_response.status());

    if let Some(length) = gateway_response.content_length() {
        if length > 5242880 {
            return Err(HttpError(502, "Content too big".to_owned()).into());
        }

        response.header(CONTENT_LENGTH, length.to_string());
    }

    for (name, value) in gateway_response.headers() {
        let lowered_name = name.as_str().to_lowercase();
        let value_str = value.to_str()?;

        if PASSTHROUGH_HEADERS.contains(&lowered_name.as_str()) {
            response.header(name.as_str(), value_str);
        }

        if lowered_name == "content-type" && !ACCEPTED_MIME_TYPES.contains(&value_str) {
            return Err(HttpError(502, "Response was not an image".to_owned()).into());
        }
    }

    Ok(response.streaming(ResponseStream {
        stream: gateway_response.bytes_stream()
    }))
}

#[derive(Deserialize)]
pub(crate) struct ProxyRequest {
    pub(crate) url: String // Hex bytes
}
