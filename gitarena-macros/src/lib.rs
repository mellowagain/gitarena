use crate::bincode::bincode as internal_bincode;
use crate::config::from_config as internal_from_config;
use crate::config::from_optional_config as internal_from_optional_config;
use crate::route::route as internal_route;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

mod bincode;
mod config;
mod route;

/// Creates resource handler, allowing multiple HTTP method guards.
/// This method is similar to the actix_web method `actix_web::route`
///
/// # Syntax
///
/// ```text
/// #[route("path", method = "HTTP_METHOD"[, attributes])]
/// ```
///
/// # Attributes
///
/// - `"path"` - Raw literal string with path for which to register handler.
/// - `method="HTTP_METHOD"` - Registers HTTP method to provide guard for. Upper-case string, "GET", "POST" for example.
/// - `guard="function_name"` - Registers function as guard using `actix_web::guard::fn_guard`
/// - `wrap="Middleware"` - Registers a resource middleware.
///
/// # Differences
///
/// 1. This macro requires `anyhow::Result` as a return type while the actix macro requires `actix_web::Result`
/// 2. This macro attaches `#[instrument(skip_all)]` from the tracing library to the function with the correct method name
///
#[proc_macro_attribute]
#[proc_macro_error]
pub fn route(args: TokenStream, input: TokenStream) -> TokenStream {
    internal_route(args, input)
}

#[proc_macro]
pub fn from_config(input: TokenStream) -> TokenStream {
    internal_from_config(input)
}

#[proc_macro]
pub fn from_optional_config(input: TokenStream) -> TokenStream {
    internal_from_optional_config(input)
}

#[proc_macro_derive(Bincode)]
pub fn derive_bincode(input: TokenStream) -> TokenStream {
    internal_bincode(input)
}
