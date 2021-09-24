use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{AttributeArgs, Error, FnArg, ItemFn, parse_macro_input, Pat};

/// Creates resource handler, allowing multiple HTTP method guards.
/// This method is similar to the actix_web method `actix_web::route`
///
/// # Syntax
/// ```text
/// #[route("path", method="HTTP_METHOD"[, attributes])]
/// ```
///
/// # Attributes
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
pub fn route(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let mut input = parse_macro_input!(input as ItemFn);

    let attrs = &mut input.attrs;
    let vis = &input.vis;
    let sig = &mut input.sig;
    let body = &input.block;

    if sig.asyncness.is_none() {
        return Error::new_spanned(sig.fn_token, "function needs to be async")
            .to_compile_error()
            .into();
    }

    // Create name for our generated function
    let ident = &sig.ident.to_string();
    let generated_ident = format!("__generated__{}", ident);
    let generated_ident_ts: TokenStream2 = generated_ident.parse().unwrap();

    let mut generated_sig = sig.clone();
    generated_sig.ident = Ident::new(generated_ident.as_str(), Span::call_site());

    // Parse list of arguments with types into list of arguments without types (just idents)
    let func_args = &sig.inputs;
    let mut idents_vec = Vec::<TokenStream2>::new();

    for func_arg in func_args {
        let ident_ts = match func_arg {
            FnArg::Typed(pat_type) => {
                let pat = &*pat_type.pat;
                match pat {
                    Pat::Ident(pat_ident) => {
                        pat_ident.ident.to_token_stream()
                    },
                    _ => unimplemented!()
                }
            },
            _ => unimplemented!()
        };
        idents_vec.push(ident_ts);
    }

    // Change from `anyhow::Result` to `actix_web::Result`
    // For this we parse a dummy function into syn and copy the output type
    // I just did this because I don't know of any other way of manually building a output type using syn
    // without including actix_web in this macro crate as well (which I want to sincerely avoid)
    let dummy_function_tokens = TokenStream::from(quote! {
        fn __generated__dummy() -> actix_web::Result<impl actix_web::Responder> {}
    });
    let mut dummy_function = parse_macro_input!(dummy_function_tokens as ItemFn);
    let dummy_signature = &mut dummy_function.sig;
    sig.output = dummy_signature.output.clone();

    TokenStream::from(quote! {
        #(#attrs)*
        #[actix_web::route(#(#args),*)]
        #[tracing::instrument(name=#ident, skip_all)]
        #vis #sig {
            #generated_sig {
                #body
            }

            Ok(#generated_ident_ts(#(#idents_vec),*).await.map_err(|e| -> crate::error::GitArenaError { e.into() }))
        }
    })
}
