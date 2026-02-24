use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Meta, Token, Type,
};

pub(crate) struct TypeWithPunctuatedMeta {
    pub(crate) ty:   Type,
    pub(crate) list: Punctuated<Meta, Token![,]>,
}

impl Parse for TypeWithPunctuatedMeta {
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ty = input.parse::<Type>()?;

        if input.is_empty() {
            return Ok(Self {
                ty,
                list: Punctuated::new(),
            });
        }

        input.parse::<Token![,]>()?;

        let list = input.parse_terminated(Meta::parse, Token![,])?;

        Ok(Self {
            ty,
            list,
        })
    }
}

#[inline]
pub(crate) fn dereference(ty: &Type) -> &Type {
    if let Type::Reference(ty) = ty {
        dereference(ty.elem.as_ref())
    } else {
        ty
    }
}

#[inline]
pub(crate) fn dereference_changed(ty: &Type) -> (&Type, bool) {
    if let Type::Reference(ty) = ty {
        (dereference(ty.elem.as_ref()), true)
    } else {
        (ty, false)
    }
}
