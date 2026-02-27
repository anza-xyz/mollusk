use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use proc_macro as pm;

struct ModQualifiers(syn::Visibility);

impl Parse for ModQualifiers {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ModQualifiers(input.parse()?))
    }
}

pub(crate) fn mod_qualifiers(meta: pm::TokenStream, item: pm::TokenStream) -> pm::TokenStream {
    let meta = syn::parse_macro_input!(meta as ModQualifiers);
    let mut item = syn::parse_macro_input!(item as syn::ItemMod);
    item.vis = meta.0;
    item.to_token_stream().into()
}
