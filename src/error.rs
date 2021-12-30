use crate::git::io::band::Band;
use crate::git::io::writer::GitWriter;

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use actix_web::dev::HttpResponseBuilder;
use actix_web::error::ResponseError;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use anyhow::Error as AnyhowError;
use anyhow::Result as AnyhowResult;
use async_compat::Compat;
use futures::executor;
use log::error;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum GAErrors {
    #[error("{1}")]
    HttpError(u16, String),

    #[error("{1}")]
    PlainError(u16, String),

    #[error("(null)")]
    GitError(u16, Option<String>),

    #[error("Unable to parse {0} from `{1}`")]
    ParseError(&'static str, String),

    #[error("Unable to unpack {0} for pack")]
    PackUnpackError(&'static str),

    #[error("Error occurred when trying to run hook: {0}")]
    HookError(&'static str),

    #[error("Type constraint was violated on {0}")]
    TypeConstraintViolated(&'static str),

    #[error("Not authenticated. Try logging in")]
    NotAuthenticated
}

impl From<GAErrors> for GitArenaError {
    fn from(ga_error: GAErrors) -> Self {
        GitArenaError {
            error: ga_error.into()
        }
    }
}

pub(crate) struct GitArenaError {
    error: AnyhowError
}

impl Display for GitArenaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.error)
    }
}

impl Debug for GitArenaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self.error)
    }
}

impl Serialize for GitArenaError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let cause = format!("{}", self.error);

        let mut state = serializer.serialize_struct("GitArenaError", 1)?;
        state.serialize_field("error", cause.as_str())?;
        state.end()
    }
}

impl From<AnyhowError> for GitArenaError {
    fn from(error: AnyhowError) -> Self {
        GitArenaError { error }
    }
}

impl ResponseError for GitArenaError {
    fn status_code(&self) -> StatusCode {
        if let Some(e) = self.error.downcast_ref::<GAErrors>() {
            match e {
                GAErrors::HttpError(status_code, _) => StatusCode::from_u16(*status_code),
                GAErrors::PlainError(status_code, _) => StatusCode::from_u16(*status_code),
                GAErrors::GitError(status_code, _) => StatusCode::from_u16(*status_code),
                GAErrors::NotAuthenticated => Ok(StatusCode::UNAUTHORIZED),

                _ => Ok(StatusCode::INTERNAL_SERVER_ERROR)
            }.unwrap_or_else(|error| {
                panic!("Invalid status code passed to GitArena error: {}", error);
            })
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Some(gitarena_error) = self.error.downcast_ref::<GAErrors>() {
            return match gitarena_error {
                GAErrors::HttpError(_, message) => {
                    HttpResponseBuilder::new(self.status_code())
                        .json(json!({
                            "error": message
                        }))
                }
                GAErrors::PlainError(_, message) => HttpResponseBuilder::new(self.status_code()).body(message),
                GAErrors::GitError(_, message_option) => {
                    match message_option {
                        Some(message) => {
                            // TODO: Refactor this to no longer block the whole thread
                            let response: AnyhowResult<HttpResponse> = executor::block_on(Compat::new(async {
                                let mut writer = GitWriter::new();
                                writer.write_text_sideband(Band::Error, format!("error: {}", message)).await?;

                                let response = writer.serialize().await?;

                                // Git doesn't show client errors if the response isn't 200 for some reason
                                Ok(HttpResponse::Ok().body(response))
                            }));

                            response.unwrap_or_else(|err| {
                                error!("In addition, another error occurred while handling the previous error: {}", err);
                                HttpResponse::InternalServerError().finish()
                            })
                        }
                        None => HttpResponseBuilder::new(self.status_code()).finish()
                    }
                },
                GAErrors::NotAuthenticated => {
                    HttpResponse::Unauthorized()
                        .json(json!({
                            "error": "Not logged in"
                        }))
                },

                _ => HttpResponse::InternalServerError().finish()
            }
        }

        HttpResponse::InternalServerError()
            .json(json!({
                "error": "Internal server error occurred"
            }))
    }
}
