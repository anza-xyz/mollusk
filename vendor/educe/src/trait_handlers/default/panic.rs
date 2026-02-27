use proc_macro2::Span;

#[inline]
pub(crate) fn multiple_default_fields(span: Span) -> syn::Error {
    syn::Error::new(span, "multiple default fields are set")
}

#[inline]
pub(crate) fn no_default_field(span: Span) -> syn::Error {
    syn::Error::new(span, "there is no field set as default")
}

#[inline]
pub(crate) fn multiple_default_variants(span: Span) -> syn::Error {
    syn::Error::new(span, "multiple default variants are set")
}

#[inline]
pub(crate) fn no_default_variant(span: Span) -> syn::Error {
    syn::Error::new(span, "there is no variant set as default")
}
