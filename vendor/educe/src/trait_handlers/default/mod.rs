mod default_enum;
mod default_struct;
mod default_union;
mod models;
mod panic;

use syn::{Data, DeriveInput, Meta};

use super::TraitHandler;
use crate::Trait;

pub(crate) struct DefaultHandler;

impl TraitHandler for DefaultHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        match ast.data {
            Data::Struct(_) => default_struct::DefaultStructHandler::trait_meta_handler(
                ast,
                token_stream,
                traits,
                meta,
            ),
            Data::Enum(_) => default_enum::DefaultEnumHandler::trait_meta_handler(
                ast,
                token_stream,
                traits,
                meta,
            ),
            Data::Union(_) => default_union::DefaultUnionHandler::trait_meta_handler(
                ast,
                token_stream,
                traits,
                meta,
            ),
        }
    }
}
