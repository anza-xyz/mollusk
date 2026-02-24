use proc_macro2::Span;
use syn::Variant;

#[inline]
pub(crate) fn multiple_deref_mut_fields(span: Span) -> syn::Error {
    syn::Error::new(span, "multiple fields are set for `DerefMut`")
}

#[inline]
pub(crate) fn multiple_deref_mut_fields_of_variant(span: Span, variant: &Variant) -> syn::Error {
    syn::Error::new(
        span,
        format!("multiple fields of the `{}` variant are set for `DerefMut`", variant.ident),
    )
}

#[inline]
pub(crate) fn no_deref_mut_field(span: Span) -> syn::Error {
    syn::Error::new(span, "there is no field which is assigned for `DerefMut`")
}

#[inline]
pub(crate) fn no_deref_mut_field_of_variant(span: Span, variant: &Variant) -> syn::Error {
    syn::Error::new(
        span,
        format!(
            "there is no field for the `{}` variant which is assigned for `DerefMut`",
            variant.ident
        ),
    )
}
