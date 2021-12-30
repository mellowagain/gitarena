use crate::{config, crypto};
use crate::sso::oauth_request::{OAuthRequest, SerdeMap};
use crate::sso::sso_provider::{DatabaseSSOProvider, SSOProvider};
use crate::sso::sso_provider_type::SSOProviderType;
use crate::user::User;
use crate::utils::identifiers::{is_username_taken, validate_username};

use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use oauth2::{AuthUrl, ClientId, ClientSecret, Scope, TokenUrl};
use reqwest::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Executor, PgPool, Postgres};
use tracing_unwrap::ResultExt;

pub(crate) struct GitHubSSO;

#[async_trait]
impl<T: DeserializeOwned> OAuthRequest<T> for GitHubSSO {
    async fn request_data(endpoint: &'static str, token: &str) -> Result<T> {
        let client = Client::new();

        Ok(client.get(format!("https://api.github.com/{}", endpoint).as_str())
            .header(ACCEPT, "application/vnd.github.v3+json")
            .header(AUTHORIZATION, format!("token {}", token))
            .header(USER_AGENT, concat!("GitArena ", env!("CARGO_PKG_VERSION")))
            .send()
            .await
            .context("Failed to connect to GitHub api")?
            .json::<T>()
            .await
            .context("Failed to parse GitHub response as JSON")?)
    }
}

#[async_trait]
impl DatabaseSSOProvider for GitHubSSO {
    async fn get_client_id<'e, E: Executor<'e, Database = Postgres>>(&self, executor: E) -> Result<ClientId> {
        let client_id = config::get_setting::<String, _>("sso.github.client_id", executor).await?;

        Ok(ClientId::new(client_id))
    }

    async fn get_client_secret<'e, E: Executor<'e, Database = Postgres>>(&self, executor: E) -> Result<Option<ClientSecret>> {
        let client_secret = config::get_setting::<String, _>("sso.github.client_secret", executor).await?;

        Ok(Some(ClientSecret::new(client_secret)))
    }
}

#[async_trait]
impl SSOProvider for GitHubSSO {
    fn get_name(&self) -> &'static str {
        "github"
    }

    fn get_auth_url(&self) -> AuthUrl {
        // unwrap_or_log() is safe as we can guarantee that this is a valid url
        AuthUrl::new("https://github.com/login/oauth/authorize".to_owned()).unwrap_or_log()
    }

    fn get_token_url(&self) -> Option<TokenUrl> {
        // unwrap_or_log() is safe as we can guarantee that this is a valid url
        Some(TokenUrl::new("https://github.com/login/oauth/access_token".to_owned()).unwrap_or_log())
    }

    fn get_scopes_as_str(&self) -> Vec<&'static str> {
        vec![
            "read:public_key", // SSH keys
            "read:user", // User profile data
            "user:email", // Emails
            "read:gpg_key", // GPG keys
        ]
    }

    fn validate_scopes(&self, scopes_option: Option<&Vec<Scope>>) -> bool {
        let granted_scopes = match scopes_option {
            Some(granted_scopes) => {
                granted_scopes
                    .iter()
                    .map(|scope| scope.split(','))
                    .flatten()
                    .collect::<Vec<_>>()
            }
            None => return true // If not provided it is identical to our asked scopes
        };

        let requested_scopes = self.get_scopes_as_str();
        granted_scopes.iter().all(|item| requested_scopes.contains(item))
    }

    async fn get_provider_id(&self, token: &str) -> Result<String> {
        let profile_data: SerdeMap = GitHubSSO::request_data("user", token).await?;

        profile_data.get("id")
            .map(|v| match v {
                Value::Number(val) => val.as_i64().map_or_else(|| None, |v| Some(v.to_string())),
                _ => None
            })
            .flatten()
            .ok_or_else(|| anyhow!("Failed to retrieve id from GitHub API json response"))
    }

    async fn create_user(&self, token: &str, db_pool: &PgPool) -> Result<User> {
        let mut transaction = db_pool.begin().await?;

        let profile_data: SerdeMap = GitHubSSO::request_data("user", token).await?;

        let mut username = profile_data.get("login")
            .map(|v| match v {
                Value::String(s) => Some(s),
                _ => None
            })
            .flatten()
            .cloned()
            .ok_or_else(|| anyhow!("Failed to retrieve username from GitHub API json response"))?;

        while validate_username(username.as_str()).is_err() || is_username_taken(username.as_str(), &mut transaction).await? {
            username = crypto::random_numeric_ascii_string(16);
        }

        let user: User = sqlx::query_as::<_, User>("insert into users (username, password) values ($1, $2) returning *")
            .bind(username.as_str())
            .bind("sso-login")
            .fetch_one(&mut transaction)
            .await?;

        let github_id = profile_data.get("id")
            .map(|v| match v {
                Value::Number(val) => val.as_i64().map_or_else(|| None, |v| Some(v.to_string())),
                _ => None
            })
            .flatten()
            .ok_or_else(|| anyhow!("Failed to retrieve id from GitHub API json response"))?;

        sqlx::query("insert into sso (user_id, provider, provider_id) values ($1, $2, $3)")
            .bind(&user.id)
            .bind(&SSOProviderType::GitHub)
            .bind(github_id.as_str())
            .execute(&mut transaction)
            .await?;

        // TODO: Save avatar (profile data "avatar_url")

        let emails: Vec<GitHubEmail> = GitHubSSO::request_data("user/emails?per_page=100", token).await?;

        for github_email in emails.iter().skip_while(|e| !e.verified) {
            let email = github_email.email.as_str();

            // Email exists
            let (email_exists,): (bool,) = sqlx::query_as("select exists(select 1 from emails where lower(email) = lower($1) limit 1)")
                .bind(email)
                .fetch_one(&mut transaction)
                .await?;

            let primary = github_email.primary;

            if email_exists {
                if primary {
                    bail!("Primary email is already assigned to a different account");
                } else {
                    continue;
                }
            }

            let public = github_email.visibility.as_ref().map_or_else(|| false, |v| v == "public");

            // All email addresses have already been verified by GitHub, so we also mark them as verified
            sqlx::query("insert into emails (owner, email, \"primary\", commit, notification, public, verified_at) values ($1, $2, $3, $3, $3, $4, current_timestamp)")
                .bind(&user.id)
                .bind(email)
                .bind(&primary)
                .bind(&public)
                .execute(&mut transaction)
                .await?;
        }

        transaction.commit().await?;

        // TODO: Save SSH keys and GPG keys

        Ok(user)
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct GitHubEmail {
    email: String,
    verified: bool,
    primary: bool,
    visibility: Option<String>
}
