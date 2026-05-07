use proc_macro::TokenStream;
use proc_macro_error::{emit_call_site_error, emit_error};
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::spanned::Spanned;
use syn::{DeriveInput, Lit, Meta, NestedMeta, parse_macro_input};

pub(crate) fn ipc_packet(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    let identifier = input.ident;

    let mut category = None;
    let mut packet_id = None;

    input.attrs.retain(|attribute| {
        if let Ok(Meta::List(list)) = attribute.parse_meta() {
            let ipc = list.path.segments.first().is_some_and(|segment| segment.ident == "ipc");

            if ipc {
                for args in list.nested {
                    if let NestedMeta::Meta(Meta::NameValue(pair)) = args
                        && let Some(segment) = pair.path.segments.first()
                    {
                        let identifier = segment.ident.to_string();
                        let value = pair.lit;

                        match identifier.as_str() {
                            "packet" => {
                                if let Lit::Str(value) = value {
                                    category = Some(value.value());
                                } else {
                                    emit_error! {
                                        value.span(),
                                        "packet requires a string argument"
                                    }
                                }
                            }
                            "id" => {
                                if let Lit::Int(value) = value {
                                    packet_id = if let Ok(id) = value.base10_parse::<u64>() {
                                        Some(id)
                                    } else {
                                        emit_error! {
                                            value.span(),
                                            "id argument could not be parsed into u64"
                                        }

                                        None
                                    };
                                } else {
                                    emit_error! {
                                        value.span(),
                                        "id requires a int argument"
                                    }
                                }
                            }
                            _ => emit_error! {
                                segment.span(),
                                "unknown identifier, expected `packet` or `id`"
                            },
                        }
                    }
                }

                return false;
            }
        }

        true
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
