use manyhow::Result;
use manyhow::{Emitter, bail, error_message};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, ExprLit, FnArg, ItemFn, Lit, LitStr, Pat, Token};

#[allow(clippy::too_many_lines)]
pub(crate) fn route(args: TokenStream, input: TokenStream, emitter: &mut Emitter) -> Result<TokenStream> {
    let mut args: Vec<Expr> = Punctuated::<Expr, Token![,]>::parse_terminated.parse(args)?.into_iter().collect();

    let mut input: ItemFn = syn::parse(input)?;

    let mut error_type = ErrorDisplayType::Unset;
    let mut error_type_index = 0;
    let mut sanitized_first_arg = None;

    for (index, meta) in args.iter().enumerate() {
        match meta {
            Expr::Assign(assign) => {
                if let Expr::Path(path) = &*assign.left
                    && let Some(segment) = path.path.segments.first()
                {
                    let lowered = segment.ident.to_string().to_lowercase();

                    if lowered.as_str() == "err"
                        && let Expr::Lit(ExprLit { lit, .. }) = &*assign.right
                        && let Some(parsed_error_type) = match_error_type(lit, emitter)
                    {
                        error_type = parsed_error_type;
                        error_type_index = index;
                    }
                } else {
                    emitter.emit(error_message!(meta.span(), "meta name cannot be empty"));
                }
            }
            Expr::Lit(ExprLit { lit: literal, .. }) if index == 0 => {
                if let Some(meta) = sanitize_first_argument(literal, emitter) {
                    sanitized_first_arg = Some(meta);
                }
            }
            _ => { /* ignored - actix web will error if the attribute is invalid */ }
        }
    }

    if matches!(error_type, ErrorDisplayType::Unset) {
        bail!(
            "function does not have \"err\" attribute";
            help = "consider adding `err = \"json|git|text|plain\"`"
        );
    }

    // actix-web doesn't know how to handle "err" so we remove it
    args.remove(error_type_index);

    // This cannot be done inline (with &mut) because of https://github.com/rust-lang/rust/issues/59159
    if let Some(meta) = sanitized_first_arg {
        args.insert(0, meta);
        args.remove(1);
    }

    // Abort right now if the previous argument parsing emitted errors
    emitter.into_result()?;

    let attrs = &mut input.attrs;
    let vis = &input.vis;
    let sig = &mut input.sig;
    let body = &input.block;

    if sig.asyncness.is_none() {
        bail!(sig.fn_token.span, "function needs to be async");
    }

    // Create name for our generated function
    let ident = &sig.ident.to_string();
    let generated_ident = format!("__generated__{ident}");
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
                    Pat::Ident(pat_ident) => pat_ident.ident.to_token_stream(),
                    _ => unimplemented!(),
                }
            }
            FnArg::Receiver(_) => unimplemented!(),
        };
        idents_vec.push(ident_ts);
    }

    let func_args = &mut sig.inputs;

    for arg in func_args {
        if let FnArg::Typed(typed_arg) = arg {
            let boxed = &mut typed_arg.pat;
            let pattern = &mut **boxed;

            if let Pat::Ident(ident) = pattern
                && ident.mutability.is_some()
            {
                ident.mutability = None;
            }
        }
    }

    // Change from `anyhow::Result` to `actix_web::Result`
    // For this we parse a dummy function into syn and copy the output type
    // I just did this because I don't know of any other way of manually building a output type using syn
    // without including actix_web in this macro crate as well (which I want to sincerely avoid)
    let dummy_function_tokens = TokenStream::from(quote! {
        fn __generated__dummy() -> actix_web::Result<impl actix_web::Responder> {}
    });
    let mut dummy_function: ItemFn = syn::parse(dummy_function_tokens)?;
    let dummy_signature = &mut dummy_function.sig;
    sig.output = dummy_signature.output.clone();

    Ok(TokenStream::from(quote! {
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
    }))
}

#[derive(Clone)]
enum ErrorDisplayType {
    Json,
    Git,
    Plain,

    #[doc(hidden)]
    Unset,
}

impl ToTokens for ErrorDisplayType {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.extend(match self {
            ErrorDisplayType::Json => quote! { Json },
            ErrorDisplayType::Git => quote! { Git },
            ErrorDisplayType::Plain => quote! { Plain },
            ErrorDisplayType::Unset => unimplemented!("unset is not mapped to a GitArena type yet"),
        });
    }
}

fn match_error_type(input: &Lit, emitter: &mut Emitter) -> Option<ErrorDisplayType> {
    if let Lit::Str(str) = input {
        let value = str.value().to_lowercase();

        return match value.as_str() {
            "json" => Some(ErrorDisplayType::Json),
            "git" => Some(ErrorDisplayType::Git),
            "text" | "plain" => Some(ErrorDisplayType::Plain),
            _ => {
                emitter.emit(error_message!(
                    input.span(),
                    "unknown error type";
                    help = "accepted types are: \"json\", \"git\", \"text\" or \"plain\""
                ));

                None
            }
        };
    }

    None
}

/// Transforms routes which are only a "/" to an empty string. This allows scoped routes to have index
/// pages without having to declare their route with a literal empty string (which is quite confusing).
fn sanitize_first_argument(literal: &Lit, emitter: &mut Emitter) -> Option<Expr> {
    if let Lit::Str(str) = literal {
        let value = str.value();

        if value.is_empty() {
            emitter.emit(error_message!(
                str.span(),
                "route cannot be empty";
                help = "if you want to match on index, use \"/\""
            ));
        } else if value == "/" {
            return Some(Expr::Lit(ExprLit {
                attrs: Vec::new(),
                lit: Lit::Str(LitStr::new("", str.span())),
            }));
        }
    }

    None
}
