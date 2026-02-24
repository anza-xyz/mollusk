use proc_macro as pm;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};

struct NamedFieldQualifier(syn::Visibility);

impl Parse for NamedFieldQualifier {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(NamedFieldQualifier(input.parse()?))
    }
}

struct NamedField(syn::Field);

impl Parse for NamedField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(NamedField(syn::Field::parse_named(input)?))
    }
}

pub(crate) fn named_field_qualifiers(meta: pm::TokenStream, item: pm::TokenStream) -> pm::TokenStream {
    let NamedFieldQualifier(vis) = syn::parse_macro_input!(meta as NamedFieldQualifier);
    let NamedField ( mut item ) = syn::parse_macro_input!(item as NamedField);
    item.vis = vis;
    item.to_token_stream().into()
}
