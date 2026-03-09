use crate::common::tools::HashType;

#[inline]
pub(crate) fn reset_a_type(ty: &HashType) -> syn::Error {
    syn::Error::new(ty.span(), format!("the type `{ty}` is repeatedly set"))
}

#[inline]
pub(crate) fn no_into_field(ty: &HashType) -> syn::Error {
    syn::Error::new(ty.span(), format!("there is no field which is assigned for `Into<{ty}>`"))
}

#[inline]
pub(crate) fn no_into_impl(ty: &HashType) -> syn::Error {
    syn::Error::new(
        ty.span(),
        format!(
            "if you want to impl `Into<{ty}>` for this type, you should write \
             `#[educe(Into({ty}))]` outside"
        ),
    )
}

#[inline]
pub(crate) fn multiple_into_fields(ty: &HashType) -> syn::Error {
    syn::Error::new(ty.span(), format!("multiple fields are set for `Into<{ty}>`"))
}
