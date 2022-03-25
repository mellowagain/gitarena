use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub(crate) fn bincode(input: TokenStream) -> TokenStream {
    let input  = parse_macro_input!(input as DeriveInput);
    let identifier = input.ident;

    let bincode = quote! {
        bincode::DefaultOptions::new()
            .with_limit(Self::max_size())
            .with_little_endian()
            .with_varint_encoding()
            .allow_trailing_bytes()
    };

    TokenStream::from(quote! {
        impl #identifier {
            #[automatically_derived]
            fn serialize(&self) -> bincode::Result<Vec<u8>> {
                use bincode::Options as _;

                #bincode.serialize(&self)
            }

            #[automatically_derived]
            fn serialize_into<W: std::io::Write>(&self, destination: W) -> bincode::Result<()> {
                use bincode::Options as _;

                #bincode.serialize_into(destination, &self)
            }

            #[automatically_derived]
            fn deserialize(input: &[u8]) -> bincode::Result<Self> {
                use bincode::Options as _;

                #bincode.deserialize::<Self>(input)
            }

            #[automatically_derived]
            fn deserialize_from<R: std::io::Read>(input: R) -> bincode::Result<Self> {
                use bincode::Options as _;

                #bincode.deserialize_from::<_, Self>(input)
            }

            #[automatically_derived]
            fn bincode_size(&self) -> bincode::Result<u64> {
                use bincode::Options as _;

                #bincode.serialized_size(&self)
            }

            /// Maximum size that this struct can be serialized from (mem::size_of::<Self> + 1 MB)
            #[automatically_derived]
            #[inline]
            const fn max_size() -> u64 {
                // Allow 1 MB additional limit
                std::mem::size_of::<Self>() as u64 + 1_000_000
            }
        }
    })
}
