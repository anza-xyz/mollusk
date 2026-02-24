mod deref_enum;
mod deref_struct;
mod models;
mod panic;

use syn::{Data, DeriveInput, Meta};

use super::TraitHandler;
use crate::Trait;

pub(crate) struct DerefHandler;

impl TraitHandler for DerefHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        match ast.data {
            Data::Struct(_) => deref_struct::DerefStructHandler::trait_meta_handler(
                ast,
                token_stream,
                traits,
                meta,
            ),
            Data::Enum(_) => {
                deref_enum::DerefEnumHandler::trait_meta_handler(ast, token_stream, traits, meta)
            },
            Data::Union(_) => {
                Err(crate::panic::trait_not_support_union(meta.path().get_ident().unwrap()))
            },
        }
    }
}
