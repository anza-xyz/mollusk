mod hash_enum;
mod hash_struct;
mod hash_union;
mod models;
mod panic;

use syn::{Data, DeriveInput, Meta};

use super::TraitHandler;
use crate::Trait;

pub(crate) struct HashHandler;

impl TraitHandler for HashHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        match ast.data {
            Data::Struct(_) => {
                hash_struct::HashStructHandler::trait_meta_handler(ast, token_stream, traits, meta)
            },
            Data::Enum(_) => {
                hash_enum::HashEnumHandler::trait_meta_handler(ast, token_stream, traits, meta)
            },
            Data::Union(_) => {
                hash_union::HashUnionHandler::trait_meta_handler(ast, token_stream, traits, meta)
            },
        }
    }
}
