use crate::config::get_setting;
use crate::database::Database;
use crate::err;
use crate::error::WithStatusCode;
use crate::prelude::USER_AGENT_STR;
use crate::sso::provider::{ProviderAuthUrlReturn, ProviderExchangeReturn, SsoProvider};
use actix_web::http::header::USER_AGENT;
use anyhow::{Result, anyhow};
use awc::ClientBuilder;
use derive_more::Display;
use oauth2::TokenResponse as _;
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata, CoreUserInfoClaims};
use openidconnect::{
    AccessTokenHash, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, SubjectIdentifier, TokenResponse,
};
use opentelemetry_instrumentation_actix_web::ClientExt;
use serde::{Deserialize, Serialize};
use sqlx::Transaction;
use tracing::instrument;

#[derive(Deserialize, Serialize)]
pub(crate) struct OidcState {
    pkce_verifier: PkceCodeVerifier,
    nonce: Nonce,
}

#[derive(Clone, Display, derive_more::Debug)]
#[display("{name}")]
pub(crate) struct Oidc {
    name: String,
    scopes: Vec<String>,
    #[debug(skip)]
    client: CoreClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointMaybeSet, EndpointMaybeSet>,
}

impl Oidc {
    #[instrument(err, skip(client_id, client_secret, scopes, tx))]
    pub(crate) async fn new(
        name: String,
        issuer_url: String,
        client_id: String,
        client_secret: String,
        scopes: Vec<String>,
        tx: &mut Transaction<'_, Database>,
    ) -> Result<Self> {
        let domain: String = get_setting("domain.app", tx).await?;

        let provider_metadata = CoreProviderMetadata::discover_async(IssuerUrl::new(issuer_url)?, &oidc_client).await?;

        let client = CoreClient::from_provider_metadata(provider_metadata, ClientId::new(client_id), Some(ClientSecret::new(client_secret)))
            .set_redirect_uri(RedirectUrl::new(format!("{domain}/api/sso/{name}/authorize"))?);

        Ok(Self { name, scopes, client })
    }
}

impl SsoProvider for Oidc {
    type State = OidcState;

    #[instrument(err)]
    async fn auth_url(&self) -> Result<ProviderAuthUrlReturn<Self::State>> {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let mut auth_req = self
            .client
            .authorize_url(CoreAuthenticationFlow::AuthorizationCode, CsrfToken::new_random, Nonce::new_random)
            .set_pkce_challenge(pkce_challenge);

        for scope in &self.scopes {
            auth_req = auth_req.add_scope(Scope::new(scope.clone()));
        }

        let (auth_url, csrf_token, nonce) = auth_req.url();

        let state = OidcState { pkce_verifier, nonce };

        Ok(ProviderAuthUrlReturn {
            csrf_token: csrf_token.into_secret(),
            auth_url: auth_url.to_string(),
            state,
        })
    }

    #[instrument(err, skip(auth_code, state))]
    async fn exchange(&self, auth_code: String, state: Self::State) -> Result<ProviderExchangeReturn> {
        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(auth_code))?
            .set_pkce_verifier(state.pkce_verifier)
            .request_async(&oidc_client)
            .await?;

        let id_token = token_response.id_token().ok_or_else(|| anyhow!("server did not return an oidc token"))?;
        let id_token_verifier = self.client.id_token_verifier();

        let claims = id_token.claims(&id_token_verifier, &state.nonce)?;

        if let Some(expected_access_token_hash) = claims.access_token_hash() {
            let actual_access_token_hash = AccessTokenHash::from_token(
                token_response.access_token(),
                id_token.signing_alg()?,
                id_token.signing_key(&id_token_verifier)?,
            )?;

            if actual_access_token_hash != *expected_access_token_hash {
                return Err(anyhow!("invalid access token"));
            }
        }

        let subject: &SubjectIdentifier = claims.subject();

        let mut username = claims.preferred_username().map(|username| username.to_string());
        let mut email = claims.email().map(|email| email.to_string());
        let mut verified = claims.email_verified();

        if username.is_none() || email.is_none() {
            let user_info: CoreUserInfoClaims = self
                .client
                .user_info(token_response.access_token().to_owned(), Some(subject.clone()))?
                .request_async(&oidc_client)
                .await?;

            username = username.or_else(|| user_info.preferred_username().map(|username| username.to_string()));
            email = email.or_else(|| user_info.email().map(|email| email.to_string()));
            verified = verified.or_else(|| user_info.email_verified());
        }

        Ok(ProviderExchangeReturn {
            subject_id: subject.to_string(),
            username: username.unwrap_or_else(|| subject.to_string()),
            emails: email.into_iter().collect(),
            emails_verified: verified.unwrap_or_default(),
        })
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

async fn oidc_client(request: openidconnect::http::Request<Vec<u8>>) -> Result<openidconnect::http::Response<Vec<u8>>, WithStatusCode> {
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
