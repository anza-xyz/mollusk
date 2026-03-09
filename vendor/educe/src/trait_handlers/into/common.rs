use quote::quote_spanned;
use syn::{spanned::Spanned, Type};

use crate::common::{r#type::dereference_changed, tools::HashType};

#[inline]
pub(crate) fn to_hash_type(ty: &Type) -> HashType {
    let (ty, is_ref) = dereference_changed(ty);

    let ty = if is_ref {
        syn::parse2(quote_spanned!( ty.span() => &'static #ty )).unwrap()
    } else {
        ty.clone()
    };

    HashType::from(ty)
}
