//! OAuth2 client implementation for the `oauth2` library using `awc` as underlying http client

use crate::err;
use crate::error::WithStatusCode;
use crate::prelude::USER_AGENT_STR;

use awc::ClientBuilder;
use awc::http::header::USER_AGENT;
use opentelemetry_instrumentation_actix_web::ClientExt;

pub(crate) async fn async_http_client(request: oauth2::HttpRequest) -> Result<oauth2::HttpResponse, WithStatusCode> {
    let client = ClientBuilder::new()
        .disable_redirects() // Following redirects opens the client up to SSRF vulnerabilities
        .add_default_header((USER_AGENT, USER_AGENT_STR))
        .finish();

    let (parts, body) = request.into_parts();

    let method = awc::http::Method::from_bytes(parts.method.as_str().as_bytes()).map_err(|err| err!(err,))?;
    let mut client_request = client.request(method, parts.uri.to_string());

    for (name, value) in &parts.headers {
        client_request = client_request.append_header((name.as_str(), value.as_bytes()));
    }

    let mut response = client_request
        .trace_request()
        .send_body(body)
        .await
        .map_err(|err| err!(BAD_GATEWAY, "Failed to contact gateway: {}", err))?;

    let response_body = response
        .body()
        .await
        .map_err(|err| err!(BAD_GATEWAY, "Failed to decode gateway response: {}", err))?;

    let mut builder = oauth2::http::Response::builder().status(response.status().as_u16());

    for (name, value) in response.headers() {
        builder = builder.header(name.as_str(), value.as_bytes());
    }

    builder
        .body(response_body.to_vec())
        .map_err(|err| err!(BAD_GATEWAY, "Invalid gateway response: {}", err))
}
