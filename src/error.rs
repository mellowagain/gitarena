use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use actix_web::dev::HttpResponseBuilder;
use actix_web::error::ResponseError;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use anyhow::Error as AnyhowError;
use log::error;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum GAErrors {
    #[error("{1}")]
    HttpError(u16, String)
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
        write!(f, "{}", self.error)
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
                GAErrors::HttpError(status_code, _) => StatusCode::from_u16(*status_code)
            }.unwrap_or(StatusCode::IM_A_TEAPOT) // A programmer passed a invalid status code
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    fn error_response(&self) -> HttpResponse {
        let message = if let Some(e) = self.error.downcast_ref::<GAErrors>() {
            match e {
                GAErrors::HttpError(_, message) => message
            }
        } else {
            "Internal server error occurred"
        };

        let status_code = self.status_code();

        if status_code.is_server_error() {
            error!("Error occurred while handling route: {}", self.error.root_cause())
        }

        let json = json!({
            "error": format!("{}", message)
        });

        HttpResponseBuilder::new(status_code).json(json)
    }
}
