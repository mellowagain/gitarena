use crate::error::{ErrorDisplayType, GitArenaError};
use crate::session::Session;
use crate::{die, err, session};

use std::convert::TryFrom;
use std::fmt::Debug;
use std::pin::Pin;
use std::sync::Arc;

use actix_identity::Identity;
use actix_web::dev::Payload;
use actix_web::web::Data;
use actix_web::{FromRequest, HttpRequest};
use anyhow::{Error, Result, anyhow};
use derive_more::Display;
use futures::Future;
use gitarena_common::database::Database;
use gitarena_common::database::Pool;
use ipnetwork::IpNetwork;
use serde::Serialize;
use sqlx::{FromRow, Transaction};
use tracing::instrument;
use tracing_unwrap::OptionExt;
use uuid::Uuid;

pub(crate) type UserId = Uuid;

#[derive(FromRow, Display, derive_more::Debug, Serialize, Clone)]
#[display("{username}")]
pub(crate) struct User {
    pub(crate) id: Uuid,
    pub(crate) username: String,
    #[serde(skip_serializing)]
    #[debug("[redacted]")]
    pub(crate) password: String,
    pub(crate) disabled: bool,
    pub(crate) admin: bool,
}

impl User {
    #[instrument(skip(tx))]
    pub(crate) async fn find_using_id(id: Uuid, tx: &mut Transaction<'_, Database>) -> Option<User> {
        sqlx::query_as::<_, User>("select * from users where id = $1 limit 1")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .ok()
            .flatten()
    }

    #[instrument(skip(tx))]
    pub(crate) async fn find_using_name<S>(name: S, tx: &mut Transaction<'_, Database>) -> Option<User>
    where
        S: AsRef<str> + Debug,
    {
        let username = name.as_ref();

        sqlx::query_as::<_, User>("select * from users where lower(username) = lower($1) limit 1")
            .bind(username)
            .fetch_optional(&mut **tx)
            .await
            .ok()
            .flatten()
    }

    #[instrument(skip(tx))]
    pub(crate) async fn find_using_email<E>(email: E, tx: &mut Transaction<'_, Database>) -> Option<User>
    where
        E: AsRef<str> + Debug,
    {
        let email = email.as_ref();

        sqlx::query_as::<_, User>("select * from users where id = (select owner from emails where lower(email) = lower($1) limit 1) limit 1")
            .bind(email)
            .fetch_optional(&mut **tx)
            .await
            .ok()
            .flatten()
    }
}

impl TryFrom<WebUser> for User {
    type Error = Error;

    fn try_from(web_user: WebUser) -> Result<Self, Self::Error> {
        web_user.into_user().map_err(|_| err!(UNAUTHORIZED).into())
    }
}

impl FromRequest for User {
    type Error = GitArenaError;
    type Future = Pin<Box<dyn Future<Output = Result<User, Self::Error>>>>;

    #[instrument(skip(_payload))]
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let match_info = req.match_info();

        // If this method gets called from a handler that does not have username or repository in the match info
        // it is safe to assume the programmer made a mistake, thus .expect_or_log is OK
        let username = match_info
            .get("user")
            .or_else(|| match_info.get("username"))
            .expect_or_log("from_request called on User despite not having user/username argument")
            .to_owned();

        match req.app_data::<Data<Pool>>() {
            Some(db_pool) => {
                let db_pool = db_pool.clone();

                Box::pin(async move {
                    extract_user_from_request(db_pool, username.as_str()).await.map_err(|err| GitArenaError {
                        source: Arc::new(err),
                        display_type: ErrorDisplayType::Json, // TODO: Check whenever route is err = "json|git" etc...
                    })
                })
            }
            None => Box::pin(async {
                Err(GitArenaError {
                    source: Arc::new(anyhow!("No PgPool in application data")),
                    display_type: ErrorDisplayType::Json, // TODO: Check whenever route is err = "json|git" etc...
                })
            }),
        }
    }
}

#[instrument(err, skip(db_pool))]
async fn extract_user_from_request(db_pool: Data<Pool>, username: &str) -> Result<User> {
    let mut transaction = db_pool.begin().await?;

    let user = User::find_using_name(username, &mut transaction)
        .await
        .ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;

    transaction.commit().await?;

    Ok(user)
}

#[derive(Debug, Display)]
pub(crate) enum WebUser {
    Anonymous,
    Authenticated(User),
}

impl WebUser {
    pub(crate) fn ok(self) -> Option<User> {
        self.into_user().ok()
    }

    pub(crate) fn as_ref(&self) -> Option<&User> {
        match self {
            WebUser::Authenticated(user) => Some(user),
            WebUser::Anonymous => None,
        }
    }

    pub(crate) fn into_user(self) -> Result<User> {
        match self {
            WebUser::Authenticated(user) => Ok(user),
            WebUser::Anonymous => die!(UNAUTHORIZED, "Not authenticated"),
        }
    }
}

impl FromRequest for WebUser {
    type Error = GitArenaError;
    type Future = Pin<Box<dyn Future<Output = Result<WebUser, Self::Error>>>>;

    #[instrument(skip(payload))]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        match req.app_data::<Data<Pool>>() {
            Some(db_pool) => {
                let (ip_network, user_agent) = session::extract_ip_and_ua_owned(req);
                let id_future = Identity::from_request(req, payload);

                let db_pool = db_pool.clone();

                Box::pin(async move {
                    extract_webuser_from_request(db_pool, id_future, ip_network, user_agent)
                        .await
                        .map_err(|err| GitArenaError {
                            source: Arc::new(err),
                            display_type: ErrorDisplayType::Json, // TODO: Check whenever route is err = "json|git" etc...
                        })
                })
            }
            None => Box::pin(async {
                Err(GitArenaError {
                    source: Arc::new(anyhow!("No PgPool in application data")),
                    display_type: ErrorDisplayType::Json, // TODO: Check whenever route is err = "json|git" etc...
                })
            }),
        }
    }
}

#[instrument(err, skip(db_pool, id_future))]
async fn extract_webuser_from_request<F: Future<Output = actix_web::Result<Identity>>>(
    db_pool: Data<Pool>,
    id_future: F,
    ip_network: IpNetwork,
    user_agent: String,
) -> Result<WebUser> {
    let id = id_future.await.map_err(|_| anyhow!("Failed to build identity"))?;

    match id.identity() {
        Some(identity) => {
            let mut transaction = db_pool.begin().await?;

            let result = if let Some(session) = Session::from_identity(Some(identity), &mut transaction).await? {
                session.update_explicit(&ip_network, user_agent.as_str(), &mut transaction).await?;

                let user: Option<User> = sqlx::query_as::<_, User>("select * from users where id = $1 limit 1")
                    .bind(session.user_id)
                    .fetch_optional(&mut *transaction)
                    .await?;

                user.map_or_else(|| WebUser::Anonymous, WebUser::Authenticated)
            } else {
                id.forget();

                WebUser::Anonymous
            };

            transaction.commit().await?;

            Ok(result)
        }
        None => Ok(WebUser::Anonymous),
    }
}
