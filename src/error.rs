use crate::git::io::band::Band;
use crate::git::io::writer::GitWriter;
use crate::templates;

use std::error::Error as StdError;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::ops::Deref;
use std::result::Result as StdResult;
use std::sync::Arc;

use actix_web::error::InternalError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, HttpResponseBuilder, ResponseError};
use anyhow::{Error, Result};
use async_compat::Compat;
use derive_more::{Display, Error};
use futures::executor;
use log::error;
use serde_json::json;
use tera::Context;

/// Returns early with an error. This macro is similar to the `bail!` macro which can be found in `anyhow`.
/// This macro is equivalent to `return Err(err!(...))`.
///
/// # Example
///
/// ```
/// # fn is_valid(input: &str) -> bool {
/// #     true
/// # }
/// #
/// # fn main() -> Result<()> {
/// #     let input = "";
/// #
/// use crate::die;
///
/// if !is_valid("input") {
///     die!(BAD_REQUEST, "Received invalid input");
/// }
/// #
/// #     Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! die {
    ($($input:tt)*) => {
        return Err($crate::err!($($input)*).into())
    }
}

/// Constructs a new error with a status code or from an existing error.
/// This macro is similar to the `anyhow!` macro which can be found in `anyhow`.
///
/// # Example
///
/// ```
/// # fn process_input(input: &str) -> Result<()> {
/// #     Ok(())
/// # }
/// #
/// # fn main() -> Result<()> {
/// #     let input = "";
/// #
/// use crate::err;
///
/// process_input(input).map_err(|_| err!(BAD_REQUEST, "Received invalid input"))?;
/// #
/// #     Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! err {
    ($code:ident) => {
        $crate::error::WithStatusCode::new(actix_web::http::StatusCode::$code)
    };
    ($code:literal) => {{
        use anyhow::Context as _;

        $crate::error::WithStatusCode::try_new($code).context("Tried to die with invalid status code")?.into()
    }};
    ($code:ident, $message:literal) => {
        $crate::error::WithStatusCode {
            code: actix_web::http::StatusCode::$code,
            source: Some(anyhow::anyhow!($message)),
            display: true
        }
    };
    ($err:expr $(,)?) => ({
        $crate::error::WithStatusCode {
            code: actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            source: Some(anyhow::anyhow!($err)),
            display: false
        }
    });
    ($code:ident, $fmt:literal, $($arg:tt)*) => {
        $crate::error::WithStatusCode {
            code: actix_web::http::StatusCode::$code,
            source: Some(anyhow::anyhow!($fmt, $($arg)*)),
            display: true
        }
    };
}

#[derive(Debug, Error)]
#[error(ignore)]
pub(crate) struct WithStatusCode {
    pub(crate) code: StatusCode,
    pub(crate) source: Option<Error>,
    pub(crate) display: bool // Whenever cause() should be shown to the user
}

impl Display for WithStatusCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match &self.source {
            Some(source) => write!(f, "http status {} caused by {:#}", self.code, source),
            None => write!(f, "http status {}", self.code)
        }
    }
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
    pub(crate) source: Arc<Error>,
    pub(crate) display_type: ErrorDisplayType
}

impl Debug for GitArenaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(self.source.deref(), f)
    }
}

impl Display for GitArenaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(self.source.deref(), f)
    }
}

impl ResponseError for GitArenaError {
    fn status_code(&self) -> StatusCode {
        match self.source.downcast_ref::<WithStatusCode>() {
            Some(with_code) => with_code.code,
            None => StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    #[allow(clippy::async_yields_async)] // False positive on this method
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
                    render_error_async(self, message.as_str()).await
                }))
            },
            ErrorDisplayType::Htmx(inner) => {
                // TODO: Send partial htmx instead
                let mut error = self.clone();
                error.display_type = *inner.clone();

                error.error_response()
            }
            ErrorDisplayType::Json => builder.json(json!({
                "error": message
            })),
            ErrorDisplayType::Plain => builder.body(message)
        }
    }
}

async fn render_error_async(renderer: &GitArenaError, message: &str) -> HttpResponse {
    match renderer.display_type {
        ErrorDisplayType::Html => render_html_error(renderer.status_code(), message).await,
        ErrorDisplayType::Git => render_git_error(message).await,
        _ => unreachable!("Only html and git errors require async handling")
    }.unwrap_or_else(|err| {
        error!("Error occurred while handling the error below! This should not happen. Please open an issue. Caused by: {}", err);
        InternalError::new(err, StatusCode::INTERNAL_SERVER_ERROR).error_response()
    })
}

async fn render_html_error(code: StatusCode, message: &str) -> Result<HttpResponse> {
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

pub(crate) trait ExtendWithStatusCode<T> {
    /// Exits early with status code on failure with `display` set to `false`
    fn code(self, status_code: StatusCode) -> StdResult<T, WithStatusCode>;

    /// Exits early with status code on failure with `display` set to `true`
    fn code_show(self, status_code: StatusCode) -> StdResult<T, WithStatusCode>;
}

impl<T, E: StdError + Send + Sync + 'static> ExtendWithStatusCode<T> for StdResult<T, E> {
    fn code(self, status_code: StatusCode) -> StdResult<T, WithStatusCode> {
        self.map_err(|err| WithStatusCode {
            code: status_code,
            source: Some(Error::from(err)),
            display: false
        })
    }

    fn code_show(self, status_code: StatusCode) -> StdResult<T, WithStatusCode> {
        self.map_err(|err| WithStatusCode {
            code: status_code,
            source: Some(Error::from(err)),
            display: true
        })
    }
}

#[derive(Display, Debug, Clone)]
pub(crate) enum ErrorDisplayType {
    Html,
    Htmx(Box<ErrorDisplayType>),
    Json,
    Git,
    Plain
}

/// Simple struct which wraps an anyhow [Error](anyhow::Error). Used in conjunction with [HoldsError] trait.
#[repr(transparent)]
pub(crate) struct ErrorHolder(pub(crate) Error);

/// Dummy trait used to constrain a template parameter to a [ErrorHolder].
pub(crate) trait HoldsError {
    /// Returns the wrapped [Error](anyhow::Error) as a value, consuming this `HoldsError` instance
    fn into_inner(self) -> Error;

    /// Returns the wrapped [Error](anyhow::Error) as a reference, leaving this `HoldsError` instance alive
    fn as_inner(&self) -> &Error;
}

impl HoldsError for ErrorHolder {
    fn into_inner(self) -> Error {
        self.0
    }

    fn as_inner(&self) -> &Error {
        &self.0
    }
}

impl From<Error> for ErrorHolder {
    fn from(err: Error) -> Self {
        ErrorHolder(err)
    }
}
