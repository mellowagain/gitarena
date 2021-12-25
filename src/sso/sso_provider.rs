use crate::config;
use crate::user::User;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::reqwest::async_http_client;
use oauth2::url::Url;
use oauth2::{AuthorizationCode, AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl};
use qstring::QString;
use sqlx::{Executor, PgPool, Postgres};
use tracing_unwrap::OptionExt;

#[async_trait]
pub(crate) trait SSOProvider {
    fn get_name(&self) -> &'static str;

    fn get_auth_url(&self) -> AuthUrl;
    fn get_token_url(&self) -> Option<TokenUrl>;

    async fn get_redirect_url<'e, E: Executor<'e, Database = Postgres>>(&self, executor: E) -> Result<RedirectUrl> {
        let domain = config::get_setting::<String, _>("domain", executor).await?;
        let url = format!("{}/sso/{}/callback", domain, self.get_name());

        Ok(RedirectUrl::new(url)?)
    }

    async fn get_client_id<'e, E: Executor<'e, Database = Postgres>>(&self, executor: E) -> Result<ClientId>;
    async fn get_client_secret<'e, E: Executor<'e, Database = Postgres>>(&self, executor: E) -> Result<Option<ClientSecret>>;

    async fn build_client(&self, db_pool: &PgPool) -> Result<BasicClient> {
        let mut transaction = db_pool.begin().await?;

        let client_id = self.get_client_id(&mut transaction).await.context("Failed to get client id")?;
        let client_secret = self.get_client_secret(&mut transaction).await.context("Failed to get client id")?;

        let auth_url = self.get_auth_url();
        let token_url = self.get_token_url();

        let redirect_url = self.get_redirect_url(&mut transaction).await.context("Failed to get redirect url")?;

        transaction.commit().await?;

        Ok(BasicClient::new(client_id, client_secret, auth_url, token_url).set_redirect_uri(redirect_url))
    }

    fn get_scopes_as_str(&self) -> Vec<&'static str>;

    fn get_scopes(&self) -> Vec<Scope> {
        self.get_scopes_as_str()
            .iter()
            .map(|scope| Scope::new(scope.to_string()))
            .collect()
    }

    async fn generate_auth_url(&self, db_pool: &PgPool) -> Result<(Url, CsrfToken)> {
        let client = self.build_client(db_pool).await?;
        let mut request = client.authorize_url(CsrfToken::new_random);

        for scope in self.get_scopes() {
            request = request.add_scope(scope);
        }

        Ok(request.url())
    }

    /// Exchanges a response (provide by `state` and `code` in `query_string`) into an oauth access token
    async fn exchange_response(&self, query_string: &QString, db_pool: &PgPool) -> Result<BasicTokenResponse> {
        let code_option = query_string.get("code");
        let state_option = query_string.get("state");

        if state_option.is_none() || state_option.is_none() {
            bail!("Received GitHub sso callback request without `code` and/or `state` in query string");
        }

        let code_str = code_option.unwrap_or_log();
        let code = AuthorizationCode::new(code_str.to_owned());

        let state_str = state_option.unwrap_or_log();
        let _state = CsrfToken::new(state_str.to_owned()); // TODO: Verify CSRF token

        let client = self.build_client(db_pool).await?;

        Ok(client.exchange_code(code)
            .request_async(async_http_client)
            .await
            .with_context(|| format!("Failed to contact {} in order to exchange oauth token", &self.get_name()))?)
    }

    /// Returns true if the granted scopes are OK or not
    fn validate_scopes(&self, scopes_option: Option<&Vec<Scope>>) -> bool {
        let granted_scopes = match scopes_option {
            Some(scopes) => scopes.iter().map(|scope| scope.as_str()).collect::<Vec<_>>(),
            None => return true // If not provided it is identical to our asked scopes
        };

        let requested_scopes = self.get_scopes_as_str();
        granted_scopes.iter().all(|item| requested_scopes.contains(item))
    }

    async fn get_provider_id(&self, token: &str) -> Result<i32>;

    async fn create_user(&self, token: &str, db_pool: &PgPool) -> Result<User>;
}
