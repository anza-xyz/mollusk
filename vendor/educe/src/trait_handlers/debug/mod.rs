mod common;
mod debug_enum;
mod debug_struct;
mod debug_union;
mod models;
mod panic;

use syn::{Data, DeriveInput, Meta};

use super::TraitHandler;
use crate::Trait;

pub(crate) struct DebugHandler;

impl TraitHandler for DebugHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        match ast.data {
            Data::Struct(_) => debug_struct::DebugStructHandler::trait_meta_handler(
                ast,
                token_stream,
                traits,
                meta,
            ),
            Data::Enum(_) => {
                debug_enum::DebugEnumHandler::trait_meta_handler(ast, token_stream, traits, meta)
            },
            Data::Union(_) => {
                debug_union::DebugUnionHandler::trait_meta_handler(ast, token_stream, traits, meta)
            },
        }
    }
}
