use std::ops::DerefMut;

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use proc_macro::TokenStream;
use proc_macro_error::{abort, abort_call_site, abort_if_dirty, emit_error};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{AttributeArgs, FnArg, ItemFn, Lit, LitStr, Meta, NestedMeta, parse_macro_input, Pat};

pub(crate) fn route(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut args = parse_macro_input!(args as AttributeArgs);
    let mut input = parse_macro_input!(input as ItemFn);

    let mut error_type = ErrorDisplayType::Unset;
    let mut error_type_index = 0;
    let mut sanitized_first_arg = None;

    for (index, meta) in args.iter().enumerate() {
        match meta {
            NestedMeta::Meta(meta) => if let Meta::NameValue(name_value) = meta {
                if let Some(segment) = name_value.path.segments.first() {
                    let lowered = segment.ident.to_string().to_lowercase();

                    if lowered.as_str() == "err" {
                        if let Some(parsed_error_type) = match_error_type(&name_value.lit) {
                            error_type = parsed_error_type;
                            error_type_index = index;
                        }
                    }
                } else {
                    emit_error! {
                        meta.span(),
                        "meta name cannot be empty"
                    }
                }
            }
            NestedMeta::Lit(literal) if index == 0 => {
                if let Some(meta) = sanitize_first_argument(literal) {
                    sanitized_first_arg = Some(meta);
                }
            }
            _ => { /* ignored - actix web will error if the attribute is invalid */ }
        }
    }

    if matches!(error_type, ErrorDisplayType::Unset) {
        abort_call_site! {
            "function does not have \"err\" attribute";
            help = "consider adding `err = \"html|htmx+(fallback)|json|git|text|plain\"`"
        }
    }

    // actix-web doesn't know how to handle "err" so we remove it
    args.remove(error_type_index);

    // This cannot be done inline (with &mut) because of https://github.com/rust-lang/rust/issues/59159
    if let Some(meta) = sanitized_first_arg {
        args.insert(0, meta);
        args.remove(1);
    }

    // Abort right now if the previous argument parsing emitted errors
    abort_if_dirty();

    let attrs = &mut input.attrs;
    let vis = &input.vis;
    let sig = &mut input.sig;
    let body = &input.block;

    if sig.asyncness.is_none() {
        abort! {
            sig.fn_token.span,
            "function needs to be async"
        }
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

    let func_args = &mut sig.inputs;

    for arg in func_args {
        match arg {
            FnArg::Typed(typed_arg) => {
                let boxed = &mut typed_arg.pat;
                let pattern = boxed.deref_mut();

                match pattern {
                    Pat::Ident(ident) if ident.mutability.is_some() => {
                        ident.mutability = None;
                    }
                    _ => { /* ignored */ }
                }
            }
            _ => { /* ignored */ }
        }
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

            Ok(#generated_ident_ts(#(#idents_vec),*).await.map_err(|err| {
                use std::sync::Arc;

                crate::error::GitArenaError {
                    source: Arc::new(err),
                    display_type: crate::error::ErrorDisplayType::#error_type
                }
            }))
        }
    })
}

#[derive(Clone)]
enum ErrorDisplayType {
    Html,
    Htmx(Box<ErrorDisplayType>),
    Json,
    Git,
    Plain,

    #[doc(hidden)]
    Unset
}

impl ToTokens for ErrorDisplayType {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.extend(match self {
            ErrorDisplayType::Html => quote! { Html },
            ErrorDisplayType::Htmx(inner) => {
                let unboxed = &*(*inner).clone();

                let ts = unboxed.to_token_stream();
                quote! { Htmx(Box::new(crate::error::ErrorDisplayType::#ts)) }
            },
            ErrorDisplayType::Json => quote! { Json },
            ErrorDisplayType::Git => quote! { Git },
            ErrorDisplayType::Plain => quote! { Plain },
            ErrorDisplayType::Unset => unimplemented!("unset is not mapped to a GitArena type yet")
        })
    }
}

fn match_error_type(input: &Lit) -> Option<ErrorDisplayType> {
    if let Lit::Str(str) = input {
        let value = str.value().to_lowercase();

        return match value.as_str() {
            "html" => Some(ErrorDisplayType::Html),
            "json" => Some(ErrorDisplayType::Json),
            "git" => Some(ErrorDisplayType::Git),
            "text" | "plain" => Some(ErrorDisplayType::Plain),
            "htmx!" => Some(ErrorDisplayType::Htmx(Box::new(ErrorDisplayType::Unset))),
            "htmx+html" => Some(ErrorDisplayType::Htmx(Box::new(ErrorDisplayType::Html))),
            "htmx+json" => Some(ErrorDisplayType::Htmx(Box::new(ErrorDisplayType::Json))),
            "htmx+git" => Some(ErrorDisplayType::Htmx(Box::new(ErrorDisplayType::Git))),
            "htmx+text" | "htmx+plain" => Some(ErrorDisplayType::Htmx(Box::new(ErrorDisplayType::Plain))),
            "htmx" => {
                emit_error! {
                    input.span(),
                    "htmx error handler requires fallback";
                    help = "if this can never happen, define err as \"htmx!\" (dangerous!)"
                }

                None
            }
            _ => {
                emit_error! {
                    input.span(),
                    "unknown error type";
                    help = "accepted types are: \"html\", \"htmx+(fallback)\", \"json\", \"git\", \"text\" or \"plain\""
                }

                None
            }
        };
    }

    None
}

/// Transforms routes which are only a "/" to an empty string. This allows scoped routes to have index
/// pages without having to declare their route with a literal empty string (which is quite confusing).
fn sanitize_first_argument(literal: &Lit) -> Option<NestedMeta> {
    if let Lit::Str(str) = literal {
        let value = str.value();

        if value.is_empty() {
            emit_error! {
                str.span(),
                "route cannot be empty";
                help = "if you want to match on index, use \"/\""
            }
        } else if value == "/" {
            return Some(NestedMeta::Lit(Lit::Str(LitStr::new("", str.span()))));
        }
    }

    None
}
