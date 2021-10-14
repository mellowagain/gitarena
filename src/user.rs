use crate::error::{GAErrors, GitArenaError};
use crate::extensions::get_user_by_identity;

use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::pin::Pin;

use actix_identity::Identity;
use actix_web::dev::Payload;
use actix_web::web::Data;
use actix_web::{FromRequest, HttpRequest, Result as ActixResult};
use anyhow::Result as AnyhowResult;
use chrono::{DateTime, Utc};
use enum_display_derive::Display;
use futures::Future;
use serde::Serialize;
use sqlx::{FromRow, PgPool};

#[derive(FromRow, Debug, Serialize)]
pub(crate) struct User {
    pub(crate) id: i32,
    pub(crate) username: String,
    pub(crate) email: String,
    pub(crate) password: String,
    pub(crate) disabled: bool,
    pub(crate) admin: bool,
    pub(crate) session: String,
    pub(crate) created_at: DateTime<Utc>
}

impl User {
    pub(crate) fn identity_str(&self) -> String {
        format!("{}${}", &self.id, &self.session)
    }
}

impl Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.username)
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
                let id_future = Identity::from_request(req, payload);
                let db_pool = db_pool.clone();

                Box::pin(async move {
                    extract_from_request(db_pool, id_future).await.map_err(|err| -> GitArenaError { err.into() })
                })
            }
            None => Box::pin(async {
                Err(GAErrors::HttpError(500, "No PgPool in application data".to_owned()).into())
            })
        }
    }
}

async fn extract_from_request<F: Future<Output = ActixResult<Identity>>>(db_pool: Data<PgPool>, id_future: F) -> AnyhowResult<WebUser> {
    let id = id_future.await.map_err(|_| GAErrors::HttpError(500, "Failed to build identity".to_owned()))?;

    match id.identity() {
        Some(identity) => {
            let user = {
                let mut transaction = db_pool.begin().await?;

                let user = get_user_by_identity(Some(identity), &mut transaction).await;

                transaction.commit().await?;

                user
            };

            Ok(user.map_or_else(|| WebUser::Anonymous, |user| WebUser::Authenticated(user)))
        }
        None => Ok(WebUser::Anonymous)
    }
}
