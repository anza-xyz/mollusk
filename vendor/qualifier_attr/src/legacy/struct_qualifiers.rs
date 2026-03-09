use syn::parse::{Parse, ParseStream};
use proc_macro as pm;
use quote::ToTokens;

struct StructQualifier(syn::Visibility);

impl Parse for StructQualifier {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(StructQualifier(input.parse()?))
    }
}

pub(crate) fn struct_qualifiers(meta: pm::TokenStream, item: pm::TokenStream) -> pm::TokenStream {
    let meta = syn::parse_macro_input!(meta as StructQualifier);
    let mut item = syn::parse_macro_input!(item as syn::ItemStruct);
    item.vis = meta.0;
    item.to_token_stream().into()
}
