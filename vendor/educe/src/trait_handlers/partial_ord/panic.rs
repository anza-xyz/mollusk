use proc_macro2::Span;

#[inline]
pub(crate) fn reuse_a_rank(span: Span, rank: isize) -> syn::Error {
    syn::Error::new(span, format!("the rank `{rank}` is repeatedly used"))
}
