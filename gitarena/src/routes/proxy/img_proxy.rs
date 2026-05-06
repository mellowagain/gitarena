use crate::prelude::{AwcExtensions, HttpRequestExtensions};
use crate::{die, err};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use crate::utils::DNS_RESOLVER;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use awc::Client;
use awc::http::StatusCode;
use awc::http::header::{CACHE_CONTROL, IF_MODIFIED_SINCE, IF_NONE_MATCH, USER_AGENT};
use gitarena_macros::route;
use opentelemetry_instrumentation_actix_web::ClientExt;
use serde::Deserialize;
use tracing::{debug, instrument};
use url::Url;

const REQUEST_PASSTHROUGH_HEADERS: &[&str] = &["if-modified-since", "if-none-match", "cache-control", "pragma"];
const RESPONSE_PASSTHROUGH_HEADERS: &[&str] = &[
    "cache-control",
    "content-length",
    "etag",
    "expires",
    "last-modified",
    "vary",
    "content-encoding",
    "transfer-encoding",
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
    "image/x-xwindowdump",
];

const NO_BODY_STATUS_CODES: &[StatusCode] = &[StatusCode::NO_CONTENT, StatusCode::RESET_CONTENT, StatusCode::NOT_MODIFIED];

#[derive(Deserialize)]
pub(crate) struct ProxyRequest {
    pub(crate) url: String, // Hex Digest
}

#[route("/api/proxy/{url}", method = "GET", err = "text")]
pub(crate) async fn proxy(uri: web::Path<ProxyRequest>, request: HttpRequest) -> Result<impl Responder> {
    let url = &uri.url;

    if url.is_empty() {
        die!(NOT_FOUND, "Invalid url");
    }

    let bytes = hex::decode(url)?;
    let string = String::from_utf8(bytes)?;

    let url = Url::parse(&string).map_err(|_| err!(BAD_REQUEST, "invalid url supplied"))?;
    let host = url.host().ok_or_else(|| err!(BAD_REQUEST, "invalid url supplied"))?;

    let lookup_ip = DNS_RESOLVER.lookup_ip(format!("{host}.")).await?;
    let mut peekable = lookup_ip.iter().peekable();

    while let Some(addr) = peekable.next() {
        if is_private_ip(addr) {
            continue;
        }

        let response = request_ip(url.as_str(), addr, url.port_or_known_default().unwrap_or(80), &request).await;
        let is_last = peekable.peek().is_none();

        if response.is_ok() || is_last {
            return response;
        }
    }

    // should theoretically never happen
    die!(BAD_GATEWAY, "unable to reach upstream server");
}

// TODO: cache this
#[instrument(skip(request))]
async fn request_ip(url: &str, address: IpAddr, port: u16, request: &HttpRequest) -> Result<HttpResponse> {
    let mut client = Client::gitarena().get(url);

    for (name, value) in request
        .headers()
        .iter()
        .map(|(name, value)| (name.as_str().to_lowercase(), value))
        .filter(|(name, _)| REQUEST_PASSTHROUGH_HEADERS.contains(&name.as_str()))
    {
        client = client.append_header((name, value));
    }

    debug!(?url, "Image proxy request");

    let upstream_response = client
        .address(SocketAddr::new(address, port))
        .timeout(Duration::from_secs(5))
        .insert_header((
            USER_AGENT,
            concat!(
                "GitArena v",
                env!("CARGO_PKG_VERSION"),
                "/ImageProxy (https://github.com/mellowagain/gitarena/)"
            ),
        ))
        .trace_request()
        .send()
        .await
        .map_err(|err| err!(BAD_GATEWAY, "Failed to send request to upstream: {err}"))?;

    let status = upstream_response.status();
    let mut response = HttpResponse::build(status);

    for (name, value) in upstream_response.headers() {
        let lowered_name = name.as_str().to_lowercase();
        let value_str = value.to_str()?;

        if RESPONSE_PASSTHROUGH_HEADERS.contains(&lowered_name.as_str()) {
            response.append_header((name.as_str(), value_str));
        }

        if lowered_name == "content-type" && !ACCEPTED_MIME_TYPES.contains(&value_str) {
            die!(BAD_GATEWAY, "Response was not an image");
        }
    }

    Ok(if NO_BODY_STATUS_CODES.contains(&status) {
        response.finish()
    } else {
        response.streaming(upstream_response)
    })
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_broadcast() || v4.is_unspecified() || (u32::from(v4) >> 22 == 0x1910_0000 >> 22) // carrier-grade NAT: 100.64.0.0/10
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // fc00::/7
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // fe80::/10
                || v6.to_ipv4_mapped().is_some_and(|v4| is_private_ip(IpAddr::V4(v4)))
        }
    }
}
