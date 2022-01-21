use crate::git::io::band::Band;
use crate::git::io::writer::GitWriter;
use crate::templates;

use std::error::Error as StdError;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::future::Future;
use std::ops::Deref;
use std::result::Result as StdResult;
use std::sync::Arc;

use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{ResponseHead, Service, ServiceRequest, ServiceResponse};
use actix_web::Error as ActixError;
use actix_web::error::InternalError;
use actix_web::http::header::{CONTENT_TYPE, HeaderValue};
use actix_web::http::StatusCode;
use actix_web::Result as ActixResult;
use actix_web::{HttpResponse, HttpResponseBuilder, ResponseError};
use anyhow::{Error, Result};
use derive_more::{Display, Error};
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

impl GitArenaError {
    fn status_code(&self) -> StatusCode {
        match self.source.downcast_ref::<WithStatusCode>() {
            Some(with_code) => with_code.code,
            None => StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    fn should_display_message(&self) -> bool {
        self.source.downcast_ref::<WithStatusCode>().map_or_else(|| false, |w| w.display)
    }

    fn message(&self) -> String {
        if self.should_display_message() {
            self.source.to_string()
        } else {
            self.status_code().canonical_reason().map_or_else(String::new, str::to_owned)
        }
    }
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
        GitArenaError::status_code(&self)
    }

    #[allow(clippy::async_yields_async)] // False positive on this method
    fn error_response(&self) -> HttpResponse {
        let mut builder = HttpResponseBuilder::new(self.status_code());

        match &self.display_type {
            ErrorDisplayType::Html | ErrorDisplayType::Git => {
                builder.extensions_mut().insert::<GitArenaError>(self.clone());

                // This method is not async which means we can't call async renders such as HTML and Git
                // As a workaround, we let a middleware (which is async) render these two error types
                // More information: https://github.com/actix/actix-web/discussions/2593
                builder.finish()
            },
            ErrorDisplayType::Htmx(inner) => {
                // TODO: Send partial htmx instead
                let mut error = self.clone();
                error.display_type = *inner.clone();

                error.error_response()
            }
            ErrorDisplayType::Json => builder.json(json!({
                "error": self.message()
            })),
            ErrorDisplayType::Plain => builder.body(self.message())
        }
    }
}

/// Middleware which renders HTML and Git errors
pub(crate) fn error_renderer_middleware<S, B>(request: ServiceRequest, service: &S) -> impl Future<Output = ActixResult<ServiceResponse<impl MessageBody>>> + 'static
    where S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
          S::Future: 'static,
          B: MessageBody + 'static
{
    let future = service.call(request);

    async {
        let mut response = future.await?.map_into_boxed_body();
        let gitarena_error = response.response_mut().extensions_mut().remove::<GitArenaError>();

        Ok(if let Some(error) = gitarena_error {
            match error.display_type {
                ErrorDisplayType::Html => {
                    let result = render_html_error(&error).await;

                    response.map_body(|head, _| {
                        head.headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"));

                        result.unwrap_or_else(|err| error_render_error(err, head))
                    })
                },
                ErrorDisplayType::Git => {
                    let result = render_git_error(&error).await;

                    response.map_body(|head, _| {
                        match result {
                            Ok(body) => {
                                // Git doesn't show client errors if the response isn't 200 for some reason
                                head.status = StatusCode::OK;
                                head.headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));

                                body
                            }
                            Err(err) => error_render_error(err, head)
                        }
                    })
                }
                _ => unreachable!("Only html and Git error responses are handled in the async middleware")
            }
        } else {
            response
        })
    }
}

async fn render_html_error(renderer: &GitArenaError) -> Result<BoxBody> {
    let mut context = Context::new();
    context.try_insert("error", renderer.message().as_str())?;

    if cfg!(debug_assertions) {
        context.try_insert("debug", &true)?;
    }

    let template_name = format!("error/{}.html", renderer.status_code().as_u16());
    let template = templates::TERA.read().await.render(template_name.as_str(), &context)?;

    Ok(BoxBody::new(template))
}

async fn render_git_error(renderer: &GitArenaError) -> Result<BoxBody> {
    let mut writer = GitWriter::new();
    writer.write_text_sideband(Band::Error, format!("error: {}", renderer.message())).await?;

    Ok(BoxBody::new(writer.serialize().await?))
}

/// Returns generic Actix 500 Internal Server Error body and logs a error message similar to Rust's ICE message.
/// This function is meant to be called when a error renderer errors.
fn error_render_error(err: Error, head: &mut ResponseHead) -> BoxBody {
    error!("| Failed to render error response: {}", err);
    error!("| GitArena encountered a error when rendering a error. This is a bug.");
    error!("| We would appreciate a bug report: https://github.com/mellowagain/gitarena/issues/new?labels=priority%3A%3Ahigh%2C+type%3A%3Acrash");

    // Fall back to the generic actix response
    let actix_response = InternalError::new(err, StatusCode::INTERNAL_SERVER_ERROR).error_response();

    head.status = StatusCode::INTERNAL_SERVER_ERROR;
    head.headers = actix_response.headers().clone();

    actix_response.into_body()
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
