use proc_macro::TokenStream;
use proc_macro_error2::{emit_call_site_error, emit_error};
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::spanned::Spanned;
use syn::{DeriveInput, LitInt, LitStr, parse_macro_input};

pub(crate) fn ipc_packet(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    let identifier = input.ident;

    let mut category = None;
    let mut packet_id = None;

    input.attrs.retain(|attribute| {
        if !attribute.path().is_ident("ipc") {
            return true;
        }

        if let Err(e) = attribute.parse_nested_meta(|meta| {
            let ident = meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
            let value = meta.value()?;

            match ident.as_str() {
                "packet" => {
                    let s: LitStr = value.parse()?;
                    category = Some(s.value());
                }
                "id" => {
                    let n: LitInt = value.parse()?;
                    match n.base10_parse::<u64>() {
                        Ok(id) => {
                            packet_id = Some(id);
                        }
                        Err(_) => {
                            emit_error!(n.span(), "id argument could not be parsed into u64");
                        }
                    }
                }
                _ => {
                    emit_error!(meta.path.span(), "unknown identifier, expected `packet` or `id`");
                }
            }

            Ok(())
        }) {
            emit_call_site_error!("{}", e);
        }

        false
    });

    if let (Some(category), Some(packet_id)) = (category, packet_id) {
        let uppercased_category = {
            // https://stackoverflow.com/a/53570840
            let mut chars = category.chars();

            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        };

        let enum_identifier = Ident::new(uppercased_category.as_str(), Span::call_site());

        TokenStream::from(quote! {
            use gitarena_macros::ipc;

            impl crate::ipc::PacketId for #identifier {
                #[inline]
                fn id(&self) -> u64 {
                    crate::packets::PacketCategory::#enum_identifier as u64 + #packet_id
                }
            }
        })
    } else {
        emit_call_site_error! {
            "#[ipc] requires both `packet` and `id` arguments";
            help = "example: #[ipc(packet = \"git\", id = 1)]";
            help = "this will result in packet id 1001 (category git = 1000 + id 1)";
        }

        TokenStream::new()
    }
}
