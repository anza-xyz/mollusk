use quote::ToTokens;
use syn::{spanned::Spanned, Meta};

#[inline]
pub(crate) fn union_without_unsafe(meta: &Meta) -> syn::Error {
    let mut s = meta.into_token_stream().to_string();

    match s.len() {
        4 => s.push_str("(unsafe)"),
        6 => s.insert_str(10, "unsafe"),
        _ => unreachable!(),
    }

    syn::Error::new(
        meta.span(),
        format!(
            "a union's `Hash` implementation is not precise, because it ignores the type of \
             fields\n* If your union doesn't care about that, use `#[educe({s})]` to implement \
             the `Hash` trait for it."
        ),
    )
}
