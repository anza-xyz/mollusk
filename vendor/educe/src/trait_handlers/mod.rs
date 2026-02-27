use syn::{DeriveInput, Meta};

use crate::Trait;

#[cfg(feature = "Clone")]
pub(crate) mod clone;
#[cfg(feature = "Copy")]
pub(crate) mod copy;
#[cfg(feature = "Debug")]
pub(crate) mod debug;
#[cfg(feature = "Default")]
pub(crate) mod default;
#[cfg(feature = "Deref")]
pub(crate) mod deref;
#[cfg(feature = "DerefMut")]
pub(crate) mod deref_mut;
#[cfg(feature = "Eq")]
pub(crate) mod eq;
#[cfg(feature = "Hash")]
pub(crate) mod hash;
#[cfg(feature = "Into")]
pub(crate) mod into;
#[cfg(feature = "Ord")]
pub(crate) mod ord;
#[cfg(feature = "PartialEq")]
pub(crate) mod partial_eq;
#[cfg(feature = "PartialOrd")]
pub(crate) mod partial_ord;

pub(crate) trait TraitHandler {
    #[allow(dead_code)]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()>;
}

pub(crate) trait TraitHandlerMultiple {
    #[allow(dead_code)]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &[Meta],
    ) -> syn::Result<()>;
}
