use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use actix_web::dev::HttpResponseBuilder;
use actix_web::error::ResponseError;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use anyhow::Error as AnyhowError;
use log::{error, warn};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum GAErrors {
    #[error("{1}")]
    HttpError(u16, String),

    #[error("(null)")]
    GitError(u16, Option<String>),

    #[error("Unable to parse {0} from `{1}`")]
    ParseError(&'static str, String),

    #[error("Unable to unpack {0} for pack")]
    PackUnpackError(&'static str)
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
                GAErrors::GitError(status_code, _) => StatusCode::from_u16(*status_code),

                _ => Ok(StatusCode::INTERNAL_SERVER_ERROR)
            }.unwrap_or_else(|error| {
                warn!("Invalid status code passed to GitArena error: {}", error);
                StatusCode::IM_A_TEAPOT
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
                GAErrors::GitError(_, message_option) => {
                    let mut response = HttpResponseBuilder::new(self.status_code());

                    if let Some(message) = message_option {
                        return response.body(message);
                    }

                    response.finish()
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
