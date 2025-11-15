use crate::prelude::AwcExtensions;
use crate::sso::oauth_request::{OAuthRequest, SerdeMap};
use crate::sso::sso_provider::{DatabaseSSOProvider, SSOProvider};
use crate::sso::sso_provider_type::SSOProviderType;
use crate::user::User;
use crate::utils::identifiers::{is_username_taken, validate_username};
use crate::{config, crypto, err};

use std::sync::Once;

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use awc::http::header::{AUTHORIZATION, USER_AGENT};
use awc::Client;
use oauth2::{AuthUrl, ClientId, ClientSecret, TokenUrl};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Executor, PgPool, Postgres};
use tracing_unwrap::ResultExt;

pub(crate) struct GitLabSSO;

#[async_trait(?Send)]
impl<T: DeserializeOwned> OAuthRequest<T> for GitLabSSO {
    async fn request_data(endpoint: &'static str, token: &str) -> Result<T> {
        let client = Client::gitarena();

        Ok(client
            .get(format!("https://gitlab.com/api/v4/{}", endpoint).as_str())
            .append_header((AUTHORIZATION, format!("Bearer {}", token)))
            .append_header((USER_AGENT, concat!("GitArena ", env!("CARGO_PKG_VERSION"))))
            .send()
            .await
            .map_err(|err| err!(BAD_GATEWAY, "Failed to connect to GitLab api: {}", err))?
            .json::<T>()
            .await
            .map_err(|err| {
                err!(
                    BAD_GATEWAY,
                    "Failed to parse GitLab response as JSON: {}",
                    err
                )
            })?)
    }
}

#[async_trait]
impl DatabaseSSOProvider for GitLabSSO {
    async fn get_client_id<'e, E: Executor<'e, Database = Postgres>>(
        &self,
        executor: E,
    ) -> Result<ClientId> {
        let client_id = config::get_setting::<String, _>("sso.gitlab.app_id", executor).await?;

        Ok(ClientId::new(client_id))
    }

    async fn get_client_secret<'e, E: Executor<'e, Database = Postgres>>(
        &self,
        executor: E,
    ) -> Result<Option<ClientSecret>> {
        let client_secret =
            config::get_setting::<String, _>("sso.gitlab.client_secret", executor).await?;

        Ok(Some(ClientSecret::new(client_secret)))
    }
}

#[async_trait(?Send)]
impl SSOProvider for GitLabSSO {
    fn get_name(&self) -> &'static str {
        "gitlab"
    }

    fn get_auth_url(&self) -> AuthUrl {
        // unwrap_or_log() is safe as we can guarantee that this is a valid url
        AuthUrl::new("https://gitlab.com/oauth/authorize".to_owned()).unwrap_or_log()
    }

    fn get_token_url(&self) -> Option<TokenUrl> {
        // unwrap_or_log() is safe as we can guarantee that this is a valid url
        Some(TokenUrl::new("https://gitlab.com/oauth/token".to_owned()).unwrap_or_log())
    }

    fn get_scopes_as_str(&self) -> Vec<&'static str> {
        vec!["read_user"]
    }

    async fn get_provider_id(&self, token: &str) -> Result<String> {
        let profile_data: SerdeMap = GitLabSSO::request_data("user", token).await?;

        profile_data
            .get("id")
            .and_then(|v| match v {
                Value::Number(val) => val.as_i64().map_or_else(|| None, |v| Some(v.to_string())),
                _ => None,
            })
            .ok_or_else(|| anyhow!("Failed to retrieve id from GitLab API json response"))
    }

    async fn create_user(&self, token: &str, db_pool: &PgPool) -> Result<User> {
        let mut transaction = db_pool.begin().await?;

        let profile_data: SerdeMap = GitLabSSO::request_data("user", token).await?;

        let mut username = profile_data
            .get("username")
            .and_then(|v| match v {
                Value::String(s) => Some(s),
                _ => None,
            })
            .cloned()
            .ok_or_else(|| anyhow!("Failed to retrieve username from GitLab API json response"))?;

        while validate_username(username.as_str()).is_err()
            || is_username_taken(username.as_str(), &mut transaction).await?
        {
            username = crypto::random_numeric_ascii_string(16);
        }

        let user: User = sqlx::query_as::<_, User>(
            "insert into users (username, password) values ($1, $2) returning *",
        )
        .bind(username.as_str())
        .bind("sso-login")
        .fetch_one(&mut transaction)
        .await?;

        let gitlab_id = profile_data
            .get("id")
            .and_then(|v| match v {
                Value::Number(val) => val.as_i64().map_or_else(|| None, |v| Some(v.to_string())),
                _ => None,
            })
            .ok_or_else(|| anyhow!("Failed to retrieve id from GitLab API json response"))?;

        sqlx::query("insert into sso (user_id, provider, provider_id) values ($1, $2, $3)")
            .bind(user.id)
            .bind(&SSOProviderType::GitLab)
            .bind(gitlab_id.as_str())
            .execute(&mut transaction)
            .await?;

        // TODO: Save avatar (profile data "avatar_url")

        let emails: Vec<GitLabEmail> = GitLabSSO::request_data("user/emails", token).await?;
        let once = Once::new();

        // For some reason GitLab does not currently always provide the `verified_at` field even for verified email addresses
        // TODO: Reactivate check once GitLab fixed their endpoint
        // Once their endpoint has been fixed, we can also mark all email addresses as verified
        for gitlab_email in emails.iter()
        /*.skip_while(|e| e.verified_at.is_none())*/
        {
            let email = gitlab_email.email.as_str();

            // Email exists
            let (email_exists,): (bool,) = sqlx::query_as(
                "select exists(select 1 from emails where lower(email) = lower($1) limit 1)",
            )
            .bind(email)
            .fetch_one(&mut transaction)
            .await?;

            if email_exists {
                continue;
            }

            let mut primary = false;

            // There is no way to get a specific primary email from the GitLab api so the first email will become the
            // primary email on GitArena. The user can also change it after the account has been created.
            once.call_once(|| {
                primary = true;
            });

            sqlx::query("insert into emails (owner, email, \"primary\", commit, notification, public) values ($1, $2, $3, $3, $3, $3)")
                .bind(user.id)
                .bind(email)
                .bind(primary)
                .execute(&mut transaction)
                .await?;

            // TODO: Send verification emails to all listed email addresses
        }

        if !once.is_completed() {
            bail!(
                "All verified GitLab email addresses are already assigned to a different account"
            );
        }

        transaction.commit().await?;

        // TODO: Save SSH keys and GPG keys

        Ok(user)
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct GitLabEmail {
    id: i32,
    email: String,
    verified_at: Option<String>,
}
