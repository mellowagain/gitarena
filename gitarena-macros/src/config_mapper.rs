use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, LitStr, Token, Type};

pub struct ConfigMappings {
    pub settings: Vec<ConfigMapping>
}

impl Parse for ConfigMappings {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut results = Vec::<ConfigMapping>::new();
        let punctuated = Punctuated::<ConfigMapping, Token![,]>::parse_terminated(input)?;

        for pair in punctuated.into_pairs() {
            let mapping = pair.into_value();
            results.push(mapping);
        }

        Ok(ConfigMappings {
            settings: results
        })
    }
}

#[derive(Clone)]
pub struct ConfigMapping {
    pub identifier: Ident,
    pub key: String,
    pub ty: Type
}

impl Parse for ConfigMapping {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: LitStr = input.parse()?;
        input.parse::<Token![=>]>()?;
        let ty: Type = input.parse()?;
        let key_str = key.value();
        let ident = Ident::new(key_str.replace(".", "_").as_str(), input.span());

        Ok(ConfigMapping {
            identifier: ident,
            key: key_str,
            ty
        })
    }
}

impl ToTokens for ConfigMapping {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.identifier;
        let ty = &self.ty;
        let key = &self.key;

        let stream = quote! {
            let #ident = crate::config::get_setting::<#ty, _>(#key, &mut transaction).await?;
        };

        tokens.extend(stream);
    }
}

/// Optional type, never gets parsed but gets created by using From<ConfigMappings>
pub struct OptionalConfigMappings {
    pub settings: Vec<OptionalConfigMapping>
}

impl From<ConfigMappings> for OptionalConfigMappings {
    fn from(mappings: ConfigMappings) -> Self {
        OptionalConfigMappings {
            settings: mappings.settings.iter().cloned().map(|c| c.into()).collect::<Vec<OptionalConfigMapping>>()
        }
    }
}

/// Optional type, never gets parsed but gets created by using From<ConfigMapping>
pub struct OptionalConfigMapping {
    pub identifier: Ident,
    pub key: String,
    pub ty: Type
}

impl From<ConfigMapping> for OptionalConfigMapping {
    fn from(mapping: ConfigMapping) -> Self {
        OptionalConfigMapping {
            identifier: mapping.identifier,
            key: mapping.key,
            ty: mapping.ty
        }
    }
}

impl ToTokens for OptionalConfigMapping {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.identifier;
        let ty = &self.ty;
        let key = &self.key;

        let stream = quote! {
            let #ident = crate::config::get_optional_setting::<#ty, _>(#key, &mut transaction).await?;
        };

        tokens.extend(stream);
    }
}
