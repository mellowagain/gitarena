use crate::error::{GAErrors, GitArenaError};
use crate::session::Session;
use crate::session;

use std::convert::TryFrom;
use std::pin::Pin;

use actix_identity::Identity;
use actix_web::dev::Payload;
use actix_web::web::Data;
use actix_web::{FromRequest, HttpRequest, Result as ActixResult};
use anyhow::Result as AnyhowResult;
use chrono::{DateTime, Utc};
use derive_more::Display;
use futures::Future;
use ipnetwork::IpNetwork;
use log::warn;
use serde::Serialize;
use sqlx::{Executor, FromRow, PgPool, Postgres};

#[derive(FromRow, Display, Debug, Serialize)]
#[display(fmt = "{}", username)]
pub(crate) struct User {
    pub(crate) id: i32,
    pub(crate) username: String,
    #[serde(skip_serializing)]
    pub(crate) password: String,
    pub(crate) disabled: bool,
    pub(crate) admin: bool,
    pub(crate) created_at: DateTime<Utc>
}

impl User {
    pub(crate) async fn find_using_name<'e, E, S>(name: S, executor: E) -> Option<User>
        where E: Executor<'e, Database = Postgres>,
              S: AsRef<str>
    {
        let username = name.as_ref();

        let user = sqlx::query_as::<_, User>("select * from users where lower(username) = lower($1) limit 1")
            .bind(username)
            .fetch_optional(executor)
            .await
            .ok()
            .flatten();

        user
    }

    pub(crate) async fn find_using_email<'e, E, S>(email: S, executor: E) -> Option<User>
        where E: Executor<'e, Database = Postgres>,
              S: AsRef<str>
    {
        let email = email.as_ref();

        let user = sqlx::query_as::<_, User>("select * from users where id = (select owner from emails where lower(email) = lower($1) limit 1) limit 1")
            .bind(email)
            .fetch_optional(executor)
            .await
            .ok()
            .flatten();

        user
    }
}

impl From<User> for i32 {
    fn from(user: User) -> i32 {
        user.id
    }
}

impl From<&User> for i32 {
    fn from(user: &User) -> i32 {
        user.id
    }
}

impl TryFrom<WebUser> for User {
    type Error = GAErrors;

    fn try_from(web_user: WebUser) -> Result<Self, Self::Error> {
        web_user.into_user().map_err(|_| GAErrors::NotAuthenticated)
    }
}

#[derive(Debug, Display)]
pub(crate) enum WebUser {
    Anonymous,
    Authenticated(User)
}

impl WebUser {
    pub(crate) fn ok(self) -> Option<User> {
        self.into_user().ok()
    }

    pub(crate) fn as_ref(&self) -> Option<&User> {
        match self {
            WebUser::Authenticated(user) => Some(user),
            WebUser::Anonymous => None
        }
    }

    pub(crate) fn into_user(self) -> AnyhowResult<User> {
        match self {
            WebUser::Authenticated(user) => Ok(user),
            WebUser::Anonymous => Err(GAErrors::NotAuthenticated.into())
        }
    }
}

impl FromRequest for WebUser {
    type Error = GitArenaError;
    type Future = Pin<Box<dyn Future<Output = Result<WebUser, Self::Error>>>>;
    type Config = ();

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        match req.app_data::<Data<PgPool>>() {
            Some(db_pool) => {
                // HttpRequest is just a wrapper around `Rc<R>` so .clone() is cheap
                let (ip_network, user_agent) = match session::extract_ip_and_ua_owned(req.clone()) {
                    Ok(tuple) => tuple,
                    Err(err) => return Box::pin(async {
                        match err.downcast::<Self::Error>() {
                            Ok(ga_error) => Err(ga_error),
                            Err(err) => {
                                warn!("Error occurred while trying to resolve user: {}", err);
                                Err(GAErrors::HttpError(500, String::new()).into())
                            }
                        }
                    })
                };

                let id_future = Identity::from_request(req, payload);

                // Data<PgPool> is just a wrapper around `Arc<P>` so .clone() is cheap
                let db_pool = db_pool.clone();

                Box::pin(async move {
                    extract_from_request(db_pool, id_future, ip_network, user_agent).await.map_err(|err| -> GitArenaError { err.into() })
                })
            }
            None => Box::pin(async {
                Err(GAErrors::HttpError(500, "No PgPool in application data".to_owned()).into())
            })
        }
    }
}

async fn extract_from_request<F: Future<Output = ActixResult<Identity>>>(db_pool: Data<PgPool>, id_future: F, ip_network: IpNetwork, user_agent: String) -> AnyhowResult<WebUser> {
    let id = id_future.await.map_err(|_| GAErrors::HttpError(500, "Failed to build identity".to_owned()))?;

    match id.identity() {
        Some(identity) => {
            let mut transaction = db_pool.begin().await?;

            let result = match Session::from_identity(Some(identity), &mut transaction).await? {
                Some(session) => {
                    session.update_explicit(&ip_network, user_agent.as_str(), &mut transaction).await?;

                    let user: Option<User> = sqlx::query_as::<_, User>("select * from users where id = $1 limit 1")
                        .bind(&session.user_id)
                        .fetch_optional(&mut transaction)
                        .await?;

                    user.map_or_else(|| WebUser::Anonymous, WebUser::Authenticated)
                }
                None => {
                    id.forget();

                    WebUser::Anonymous
                }
            };

            transaction.commit().await?;

            Ok(result)
        }
        None => Ok(WebUser::Anonymous)
    }
}
