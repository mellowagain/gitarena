use crate::{config, crypto};
use crate::sso::oauth_request::{OAuthRequest, SerdeMap};
use crate::sso::sso_provider::{DatabaseSSOProvider, SSOProvider};
use crate::sso::sso_provider_type::SSOProviderType;
use crate::user::User;
use crate::utils::identifiers::{is_username_taken, validate_username};

use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use oauth2::{AuthUrl, ClientId, ClientSecret, TokenUrl};
use reqwest::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Executor, PgPool, Postgres};
use tokio_compat_02::FutureExt;
use tracing_unwrap::ResultExt;

pub(crate) struct BitBucketSSO;

#[async_trait]
impl<T: DeserializeOwned> OAuthRequest<T> for BitBucketSSO {
    async fn request_data(endpoint: &'static str, token: &str) -> Result<T> {
        let client = Client::new();

        Ok(client.get(format!("https://api.bitbucket.org/2.0/{}", endpoint).as_str())
            .header(ACCEPT, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .header(USER_AGENT, concat!("GitArena ", env!("CARGO_PKG_VERSION")))
            .send()
            .compat()
            .await
            .context("Failed to connect to BitBucket api")?
            .json::<T>()
            .compat()
            .await
            .context("Failed to parse BitBucket response as JSON")?)
    }
}

#[async_trait]
impl DatabaseSSOProvider for BitBucketSSO {
    async fn get_client_id<'e, E: Executor<'e, Database = Postgres>>(&self, executor: E) -> Result<ClientId> {
        let client_id = config::get_setting::<String, _>("sso.bitbucket.key", executor).await?;

        Ok(ClientId::new(client_id))
    }

    async fn get_client_secret<'e, E: Executor<'e, Database = Postgres>>(&self, executor: E) -> Result<Option<ClientSecret>> {
        let client_secret = config::get_setting::<String, _>("sso.bitbucket.secret", executor).await?;

        Ok(Some(ClientSecret::new(client_secret)))
    }
}

#[async_trait]
impl SSOProvider for BitBucketSSO {
    fn get_name(&self) -> &'static str {
        "bitbucket"
    }

    fn get_auth_url(&self) -> AuthUrl {
        // unwrap_or_log() is safe as we can guarantee that this is a valid url
        AuthUrl::new("https://bitbucket.org/site/oauth2/authorize".to_owned()).unwrap_or_log()
    }

    fn get_token_url(&self) -> Option<TokenUrl> {
        // unwrap_or_log() is safe as we can guarantee that this is a valid url
        Some(TokenUrl::new("https://bitbucket.org/site/oauth2/access_token".to_owned()).unwrap_or_log())
    }

    fn get_scopes_as_str(&self) -> Vec<&'static str> {
        vec![
            "account",
            "email"
        ]
    }

    async fn get_provider_id(&self, token: &str) -> Result<String> {
        let profile_data: SerdeMap = BitBucketSSO::request_data("user", token).await?;

        profile_data.get("account_id")
            .map(|v| match v {
                Value::String(val) => Some(val.to_owned()),
                _ => None
            })
            .flatten()
            .ok_or_else(|| anyhow!("Failed to retrieve id from BitBucket API json response"))
    }

    async fn create_user(&self, token: &str, db_pool: &PgPool) -> Result<User> {
        let mut transaction = db_pool.begin().await?;

        let profile_data: SerdeMap = BitBucketSSO::request_data("user", token).await?;

        let mut username = profile_data.get("username")
            .map(|v| match v {
                Value::String(s) => Some(s),
                _ => None
            })
            .flatten()
            .cloned()
            .ok_or_else(|| anyhow!("Failed to retrieve username from BitBucket API json response"))?;

        while validate_username(username.as_str()).is_err() || is_username_taken(username.as_str(), &mut transaction).await? {
            username = crypto::random_numeric_ascii_string(16);
        }

        let user: User = sqlx::query_as::<_, User>("insert into users (username, password) values ($1, $2) returning *")
            .bind(username.as_str())
            .bind("sso-login")
            .fetch_one(&mut transaction)
            .await?;

        let bitbucket_id = profile_data.get("account_id")
            .map(|v| match v {
                Value::String(val) => Some(val.to_owned()),
                _ => None
            })
            .flatten()
            .ok_or_else(|| anyhow!("Failed to retrieve id from BitBucket API json response"))?;

        sqlx::query("insert into sso (user_id, provider, provider_id) values ($1, $2, $3)")
            .bind(&user.id)
            .bind(&SSOProviderType::BitBucket)
            .bind(bitbucket_id.as_str())
            .execute(&mut transaction)
            .await?;

        // TODO: Save avatar (profile data "avatar_url")

        let emails: BitBucketEmailList = BitBucketSSO::request_data("user/emails", token).await?;

        for bitbucket_email in emails.values.iter().skip_while(|e| !e.is_confirmed || e.email_type != "email") {
            let email = bitbucket_email.email.as_str();

            // Email exists
            let (email_exists,): (bool,) = sqlx::query_as("select exists(select 1 from emails where lower(email) = lower($1) limit 1)")
                .bind(email)
                .fetch_one(&mut transaction)
                .await?;

            let primary = bitbucket_email.is_primary;

            if email_exists {
                if primary {
                    bail!("Primary email is already assigned to a different account");
                } else {
                    continue;
                }
            }

            // All email addresses have already been verified by Bitbucket, so we also mark them as verified
            sqlx::query("insert into emails (owner, email, \"primary\", commit, notification, public, verified_at) values ($1, $2, $3, $3, $3, $3, current_timestamp)")
                .bind(&user.id)
                .bind(email)
                .bind(&primary)
                .execute(&mut transaction)
                .await?;
        }

        transaction.commit().await?;

        // TODO: Save SSH keys and GPG keys

        Ok(user)
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct BitBucketEmailList {
    #[serde(rename(deserialize = "pagelen"))]
    page_length: usize,
    values: Vec<BitBucketEmail>,
    page: usize,
    size: usize
}

#[derive(Deserialize, Serialize, Debug)]
struct BitBucketEmail {
    is_primary: bool,
    is_confirmed: bool,
    #[serde(rename(deserialize = "type"))]
    email_type: String,
    email: String,
    #[serde(skip_deserializing)]
    links: Option<Value>
}
