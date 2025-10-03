//! OAuth2 client implementation for the `oauth2` library using `awc` as underlying http client

use crate::err;
use crate::error::WithStatusCode;
use crate::prelude::USER_AGENT_STR;

use awc::http::header::USER_AGENT;
use awc::ClientBuilder;
use oauth2::{HttpRequest, HttpResponse};

pub(crate) async fn async_http_client(
    request: HttpRequest,
) -> Result<HttpResponse, WithStatusCode> {
    let client = ClientBuilder::new()
        .disable_redirects() // Following redirects opens the client up to SSRF vulnerabilities
        .add_default_header((USER_AGENT, USER_AGENT_STR))
        .finish();

    let mut client_request = client.request(request.method, request.url.as_str());

    for pair in &request.headers {
        client_request = client_request.append_header(pair);
    }

    let mut response = client_request
        .send_body(request.body)
        .await
        .map_err(|err| err!(BAD_GATEWAY, "Failed to contact gateway: {}", err))?;

    let response_body = response
        .body()
        .await
        .map_err(|err| err!(BAD_GATEWAY, "Failed to decode gateway response: {}", err))?;

    Ok(HttpResponse {
        status_code: response.status(),
        headers: request.headers,
        body: response_body.to_vec(),
    })
}
