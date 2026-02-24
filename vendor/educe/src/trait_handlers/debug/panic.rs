use quote::ToTokens;
use syn::{spanned::Spanned, Ident, Meta, Variant};

#[inline]
pub(crate) fn unit_struct_need_name(name: &Ident) -> syn::Error {
    syn::Error::new(name.span(), "a unit struct needs to have a name")
}

#[inline]
pub(crate) fn unit_variant_need_name(variant: &Variant) -> syn::Error {
    syn::Error::new(
        variant.span(),
        "a unit variant which doesn't use an enum name needs to have a name",
    )
}

#[inline]
pub(crate) fn unit_enum_need_name(name: &Ident) -> syn::Error {
    syn::Error::new(name.span(), "a unit enum needs to have a name")
}

#[inline]
pub(crate) fn union_without_unsafe(meta: &Meta) -> syn::Error {
    let mut s = meta.into_token_stream().to_string().replace(" , ", ", ");

    match s.len() {
        5 => s.push_str("(unsafe)"),
        7 => s.insert_str(6, "unsafe"),
        _ => s.insert_str(6, "unsafe, "),
    }

    syn::Error::new(
        meta.span(),
        format!(
            "a union's `Debug` implementation may expose uninitialized memory\n* It is \
             recommended that, for a union where `Debug` is implemented, types that allow \
             uninitialized memory should not be used in it.\n* If you can ensure that the union \
             uses no such types, use `#[educe({s})]` to implement the `Debug` trait for it.\n* \
             The `unsafe` keyword should be placed as the first parameter of the `Debug` \
             attribute."
        ),
    )
}
