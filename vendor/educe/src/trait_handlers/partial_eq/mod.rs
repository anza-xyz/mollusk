mod models;
mod panic;
mod partial_eq_enum;
mod partial_eq_struct;
mod partial_eq_union;

use syn::{Data, DeriveInput, Meta};

use super::TraitHandler;
use crate::Trait;

pub(crate) struct PartialEqHandler;

impl TraitHandler for PartialEqHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        match ast.data {
            Data::Struct(_) => partial_eq_struct::PartialEqStructHandler::trait_meta_handler(
                ast,
                token_stream,
                traits,
                meta,
            ),
            Data::Enum(_) => partial_eq_enum::PartialEqEnumHandler::trait_meta_handler(
                ast,
                token_stream,
                traits,
                meta,
            ),
            Data::Union(_) => partial_eq_union::PartialEqUnionHandler::trait_meta_handler(
                ast,
                token_stream,
                traits,
                meta,
            ),
        }
    }
}
