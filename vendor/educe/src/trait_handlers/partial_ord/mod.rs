mod models;
mod panic;
mod partial_ord_enum;
mod partial_ord_struct;

use models::TypeAttributeBuilder;
use syn::{Data, DeriveInput, Meta};

use super::TraitHandler;
use crate::Trait;

pub(crate) struct PartialOrdHandler;

impl TraitHandler for PartialOrdHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        #[cfg(feature = "Ord")]
        let contains_ord = traits.contains(&Trait::Ord);

        #[cfg(not(feature = "Ord"))]
        let contains_ord = false;

        // if `contains_ord` is true, the implementation is handled by the `Ord` attribute
        if contains_ord {
            let _ = TypeAttributeBuilder {
                enable_flag: true, enable_bound: false
            }
            .build_from_partial_ord_meta(meta)?;

            // field attributes is also handled by the `Ord` attribute

            Ok(())
        } else {
            match ast.data {
                Data::Struct(_) => partial_ord_struct::PartialOrdStructHandler::trait_meta_handler(
                    ast,
                    token_stream,
                    traits,
                    meta,
                ),
                Data::Enum(_) => partial_ord_enum::PartialOrdEnumHandler::trait_meta_handler(
                    ast,
                    token_stream,
                    traits,
                    meta,
                ),
                Data::Union(_) => {
                    Err(crate::panic::trait_not_support_union(meta.path().get_ident().unwrap()))
                },
            }
        }
    }
}
