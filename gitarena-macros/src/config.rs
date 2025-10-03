use std::fmt::{Debug, Formatter};

use proc_macro::TokenStream as ProcMacroTS;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Ident, LitStr, Token, Type};

pub(crate) fn from_config(input: ProcMacroTS) -> ProcMacroTS {
    let settings = parse_macro_input!(input as SettingsList);
    let identifiers = settings.identifiers();

    ProcMacroTS::from(quote! {{
        let mut transaction = db_pool.begin().await?;

        #settings

        transaction.commit().await?;

        (#(#identifiers),*)
    }})
}

pub(crate) fn from_optional_config(input: ProcMacroTS) -> ProcMacroTS {
    let settings = parse_macro_input!(input as SettingsList);
    let identifiers = settings.identifiers();
    let settings = settings.as_optional();

    ProcMacroTS::from(quote! {{
        let mut transaction = db_pool.begin().await?;

        #settings

        transaction.commit().await?;

        (#(#identifiers),*)
    }})
}

// Required config

#[derive(Debug)]
struct SettingsList {
    settings: Vec<Setting>,
}

impl SettingsList {
    fn identifiers(&self) -> Vec<&Ident> {
        self.settings.iter().map(|s| &s.identifier).collect()
    }

    fn as_optional(&self) -> OptionalSettingsList {
        OptionalSettingsList {
            settings: self
                .settings
                .iter()
                .cloned()
                .map(|s| s.as_optional())
                .collect(),
        }
    }
}

impl Parse for SettingsList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let punctuated = Punctuated::<Setting, Token![,]>::parse_terminated(input)?;

        Ok(SettingsList {
            settings: punctuated.iter().cloned().collect::<Vec<Setting>>(),
        })
    }
}

impl ToTokens for SettingsList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let settings = &self.settings;

        let stream = quote! {
            #(#settings)*
        };

        tokens.extend(stream);
    }
}

#[derive(Clone)]
struct Setting {
    identifier: Ident,
    key: String,
    ty: Type,
}

impl Debug for Setting {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Setting")
            .field("identifier", &self.identifier)
            .field("key", &self.key)
            .finish()
    }
}

impl Setting {
    fn as_optional(&self) -> OptionalSetting {
        OptionalSetting {
            original: self.clone(),
        }
    }
}

impl Parse for Setting {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: LitStr = input.parse()?;
        input.parse::<Token![=>]>()?;
        let ty: Type = input.parse()?;
        let key_str = key.value();
        let ident = Ident::new(key_str.replace('.', "_").as_str(), input.span());

        Ok(Setting {
            identifier: ident,
            key: key_str,
            ty,
        })
    }
}

impl ToTokens for Setting {
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

// Optional config

#[derive(Debug)]
struct OptionalSettingsList {
    settings: Vec<OptionalSetting>,
}

impl ToTokens for OptionalSettingsList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let settings = &self.settings;

        let stream = quote! {
            #(#settings)*
        };

        tokens.extend(stream);
    }
}

#[derive(Debug)]
struct OptionalSetting {
    original: Setting,
}

impl ToTokens for OptionalSetting {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.original.identifier;
        let ty = &self.original.ty;
        let key = &self.original.key;

        let stream = quote! {
            let #ident = crate::config::get_optional_setting::<#ty, _>(#key, &mut transaction).await?;
        };

        tokens.extend(stream);
    }
}
