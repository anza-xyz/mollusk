mod models;
mod ord_enum;
mod ord_struct;
mod panic;

use quote::quote;
use syn::{Data, DeriveInput, Meta};

use super::TraitHandler;
use crate::Trait;

pub(crate) struct OrdHandler;

impl TraitHandler for OrdHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        match ast.data {
            Data::Struct(_) => {
                ord_struct::OrdStructHandler::trait_meta_handler(ast, token_stream, traits, meta)
            },
            Data::Enum(_) => {
                ord_enum::OrdEnumHandler::trait_meta_handler(ast, token_stream, traits, meta)
            },
            Data::Union(_) => {
                Err(crate::panic::trait_not_support_union(meta.path().get_ident().unwrap()))
            },
        }
    }
}

fn supertraits(#[allow(unused_variables)] traits: &[Trait]) -> Vec<proc_macro2::TokenStream> {
    let mut supertraits = vec![];
    supertraits.push(quote! {::core::cmp::Eq});

    // We mustn't add the PartialOrd bound to the educed PartialOrd impl.
    // When we're educing PartialOrd we can leave it off the Ord impl too,
    // since we *know* Self is going to be PartialOrd.
    #[cfg(feature = "PartialOrd")]
    if !traits.contains(&Trait::PartialOrd) {
        supertraits.push(quote! {::core::cmp::PartialOrd});
    };

    supertraits
}
