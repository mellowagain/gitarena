use crate::git::io::band::Band;
use crate::git::io::writer::GitWriter;
use crate::templates;

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use actix_web::dev::HttpResponseBuilder;
use actix_web::error::{InternalError, PrivateHelper};
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use anyhow::{Error, Result};
use async_compat::Compat;
use derive_more::{Display, Error};
use futures::executor;
use log::error;
use serde_json::json;
use tera::Context;

macro_rules! die {
    ($code:expr) => {
        return Err($crate::error::WithStatusCode::new(actix_web::http::StatusCode::$code).into());
    }
    ($code:literal) => {{
        use anyhow::Context as _;

        return Err($crate::error::WithStatusCode::try_new($code).context("Tried to die with invalid status code")?.into());
    }}
    ($code:expr, $message:literal) => {
        return Err($crate::error::WithStatusCode {
            code: actix_web::http::StatusCode::$code,
            source: anyhow::anyhow!($message),
            display: false
        })
    }
    ($err:expr $(,)?) => ({
        return Err($crate::error::WithStatusCode {
            code: actix_web::http::StatusCode::$code,
            source: anyhow::anyhow!($err),
            display: false
        })
    })
    ($code:expr, $fmt:literal, $($arg:tt)*) => {
        return Err($crate::error::WithStatusCode {
            code: actix_web::http::StatusCode::$code,
            source: anyhow::anyhow!($fmt, $($arg)*),
            display: true
        })
    }
}

#[derive(Debug, Display, Error)]
#[display("http status {} caused by {}", code, source)]
pub(crate) struct WithStatusCode {
    code: StatusCode,
    source: Option<Error>,
    display: bool // Whenever cause() should be shown to the user
}

impl WithStatusCode {
    pub(crate) fn new(code: StatusCode) -> WithStatusCode {
        WithStatusCode {
            code,
            source: None,
            display: false
        }
    }

    pub(crate) fn try_new(code: u16) -> Result<WithStatusCode> {
        Ok(WithStatusCode {
            code: StatusCode::from_u16(code)?,
            source: None,
            display: false
        })
    }
}

#[derive(Clone)]
pub(crate) struct GitArenaError {
    source: Error,
    display_type: ErrorDisplayType
}

impl Debug for GitArenaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&self.source, f)
    }
}

impl Display for GitArenaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.source, f)
    }
}

impl ResponseError for GitArenaError {
    fn status_code(&self) -> StatusCode {
        match self.source.downcast_ref::<WithStatusCode>() {
            Some(with_code) => with_code.code,
            None => StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    fn error_response(&self) -> HttpResponse {
        let mut builder = HttpResponseBuilder::new(self.status_code());

        let display = self.source.downcast_ref::<WithStatusCode>().map_or_else(|| false, |w| w.display);
        let message = if display {
            self.source.to_string()
        } else {
            self.status_code().canonical_reason().unwrap_or_default().to_string()
        };

        match &self.display_type {
            ErrorDisplayType::Html | ErrorDisplayType::Git => {
                // TODO: Refactor this to no longer block the whole thread
                executor::block_on(Compat::new(async {
                    render_error_async(&self, builder, message.as_str()).await
                }))
            },
            ErrorDisplayType::Htmx(inner) => {
                // TODO: Send partial htmx instead
                let mut error = self.clone();
                error.display_type = **inner;

                error.error_response()
            }
            ErrorDisplayType::Json => builder.json(json!({
                "error": message
            })),
            ErrorDisplayType::Plain => builder.body(message)
        }
    }
}

async fn render_error_async(renderer: &GitArenaError, builder: HttpResponseBuilder, message: &str) -> HttpResponse {
    match renderer.display_type {
        ErrorDisplayType::Html => render_html_error(renderer.status_code(), message).await,
        ErrorDisplayType::Git => render_git_error(message).await,
        _ => unreachable!("Only html and git errors require async handling")
    }.unwrap_or_else(|err| {
        error!("In addition, another error occurred while handling the previous error: {}", err);
        InternalError::new(err, StatusCode::INTERNAL_SERVER_ERROR).into()
    })
}

async fn render_html_error(code: StatusCode, message: &str, display: bool) -> Result<HttpResponse> {
    let mut context = Context::new();
    context.try_insert("error", message)?;

    if cfg!(debug_assertions) {
        context.try_insert("debug", &true)?;
    }

    let template_name = format!("error/{}.html", code.as_u16());
    let template = templates::TERA.read().await.render(template_name.as_str(), &context)?;

    Ok(HttpResponseBuilder::new(code).body(template))
}

async fn render_git_error(message: &str) -> Result<HttpResponse> {
    let mut writer = GitWriter::new();
    writer.write_text_sideband(Band::Error, format!("error: {}", message)).await?;

    let response = writer.serialize().await?;

    // Git doesn't show client errors if the response isn't 200 for some reason
    Ok(HttpResponse::Ok().body(response))
}

pub(crate) enum ErrorDisplayType {
    Html,
    Htmx(Box<ErrorDisplayType>),
    Json,
    Git,
    Plain
}
