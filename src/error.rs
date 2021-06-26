use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use actix_web::error::ResponseError;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use anyhow::Error as AnyhowError;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

pub struct GitArenaError {
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
        //let s = format!("{}", self.error);
        //let sr: &str = s.as_str();

        let mut state = serializer.serialize_struct("GitArenaError", 1)?;
        state.serialize_field("error", /*sr*/"Internal server error occurred")?;
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
        //if let Some(a) = self.err.downcast_ref::<Error>() {
        //}

        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::InternalServerError().json(self)
    }
}
