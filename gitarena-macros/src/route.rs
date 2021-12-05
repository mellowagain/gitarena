use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use proc_macro::TokenStream;
use proc_macro_error::abort;
use quote::{quote, ToTokens};
use syn::{AttributeArgs, Error, FnArg, ItemFn, Lit, LitStr, NestedMeta, parse_macro_input, Pat};

pub(crate) fn route(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut args = parse_macro_input!(args as AttributeArgs);
    let mut input = parse_macro_input!(input as ItemFn);

    // Transform routes which are only a "/" to an empty string. This allows scoped routes to have index
    // pages without having to declare their route with a literal empty string (which is quite confusing).
    // This cannot be done inline because of https://github.com/rust-lang/rust/issues/59159,
    // so we return a tuple which allows us to mutually borrow later if needed.
    let (sanitize_slash, span) = if let Some(first_arg) = args.first() {
        match first_arg {
            NestedMeta::Lit(literal) => match literal {
                Lit::Str(str) => {
                    let value = str.value();

                    if value.is_empty() {
                        abort! {
                            str.span(),
                            "route cannot be empty";
                            help = "if you want to match on index, use \"/\"";
                        }
                    } else if value == "/" {
                        (true, Some(str.span()))
                    } else {
                        (false, None)
                    }
                }
                _ => (false, None)
            }
            NestedMeta::Meta(_) => (false, None)
        }
    } else {
        (false, None)
    };

    if sanitize_slash {
        args.insert(0, NestedMeta::Lit(Lit::Str(LitStr::new("", span.unwrap()))));
        args.remove(1);
    }

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