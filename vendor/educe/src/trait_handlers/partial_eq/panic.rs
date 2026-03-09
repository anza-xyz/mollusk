use quote::ToTokens;
use syn::{spanned::Spanned, Meta};

#[inline]
pub(crate) fn union_without_unsafe(meta: &Meta) -> syn::Error {
    let mut s = meta.into_token_stream().to_string();

    match s.len() {
        9 => s.push_str("(unsafe)"),
        11 => s.insert_str(10, "unsafe"),
        _ => unreachable!(),
    }

    syn::Error::new(
        meta.span(),
        format!(
            "a union's `PartialEq` implementation is not precise, because it ignores the type of \
             fields\n* If your union doesn't care about that, use `#[educe({s})]` to implement \
             the `PartialEq` trait for it."
        ),
    )
}
