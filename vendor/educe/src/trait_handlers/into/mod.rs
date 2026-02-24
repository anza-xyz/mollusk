mod common;
mod into_enum;
mod into_struct;
mod models;
mod panic;

use syn::{Data, DeriveInput, Meta};

use super::TraitHandlerMultiple;
use crate::Trait;

pub(crate) struct IntoHandler;

impl TraitHandlerMultiple for IntoHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &[Meta],
    ) -> syn::Result<()> {
        match ast.data {
            Data::Struct(_) => {
                into_struct::IntoStructHandler::trait_meta_handler(ast, token_stream, traits, meta)
            },
            Data::Enum(_) => {
                into_enum::IntoEnumHandler::trait_meta_handler(ast, token_stream, traits, meta)
            },
            Data::Union(_) => {
                Err(crate::panic::trait_not_support_union(meta[0].path().get_ident().unwrap()))
            },
        }
    }
}
