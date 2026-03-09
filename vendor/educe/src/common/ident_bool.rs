use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Expr, Ident, Lit, LitBool, LitStr, Meta, MetaNameValue,
};

use super::path::path_to_string;

#[derive(Debug)]
pub(crate) enum IdentOrBool {
    Ident(Ident),
    Bool(bool),
}

impl Parse for IdentOrBool {
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Ok(lit) = input.parse::<Lit>() {
            match lit {
                Lit::Bool(lit) => return Ok(Self::Bool(lit.value)),
                Lit::Str(lit) => {
                    return match lit.parse::<Ident>() {
                        Ok(ident) => Ok(Self::Ident(ident)),
                        Err(_) if lit.value().is_empty() => Ok(Self::Bool(false)),
                        Err(error) => Err(error),
                    }
                },
                _ => (),
            }
        }

        Ok(Self::Ident(input.parse::<Ident>()?))
    }
}

#[inline]
pub(crate) fn meta_name_value_2_ident(name_value: &MetaNameValue) -> syn::Result<Ident> {
    match &name_value.value {
        Expr::Lit(lit) => {
            if let Lit::Str(lit) = &lit.lit {
                return lit.parse();
            }
        },
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                return Ok(ident.clone());
            }
        },
        _ => (),
    }

    Err(syn::Error::new(
        name_value.value.span(),
        format!("expected `{path} = Ident`", path = path_to_string(&name_value.path)),
    ))
}

#[inline]
pub(crate) fn meta_2_ident(meta: &Meta) -> syn::Result<Ident> {
    match &meta {
        Meta::NameValue(name_value) => meta_name_value_2_ident(name_value),
        Meta::List(list) => {
            if let Ok(lit) = list.parse_args::<LitStr>() {
                lit.parse()
            } else {
                list.parse_args()
            }
        },
        Meta::Path(path) => Err(syn::Error::new(
            path.span(),
            format!("expected `{path} = Ident` or `{path}(Ident)`", path = path_to_string(path)),
        )),
    }
}

#[inline]
pub(crate) fn meta_name_value_2_bool(name_value: &MetaNameValue) -> syn::Result<bool> {
    if let Expr::Lit(lit) = &name_value.value {
        if let Lit::Bool(b) = &lit.lit {
            return Ok(b.value);
        }
    }

    Err(syn::Error::new(
        name_value.value.span(),
        format!("expected `{path} = false`", path = path_to_string(&name_value.path)),
    ))
}

#[inline]
pub(crate) fn meta_2_bool(meta: &Meta) -> syn::Result<bool> {
    match &meta {
        Meta::NameValue(name_value) => meta_name_value_2_bool(name_value),
        Meta::List(list) => Ok(list.parse_args::<LitBool>()?.value),
        Meta::Path(path) => Err(syn::Error::new(
            path.span(),
            format!("expected `{path} = false` or `{path}(false)`", path = path_to_string(path)),
        )),
    }
}

#[inline]
pub(crate) fn meta_2_bool_allow_path(meta: &Meta) -> syn::Result<bool> {
    match &meta {
        Meta::Path(_) => Ok(true),
        Meta::NameValue(name_value) => meta_name_value_2_bool(name_value),
        Meta::List(list) => Ok(list.parse_args::<LitBool>()?.value),
    }
}

#[inline]
pub(crate) fn meta_name_value_2_ident_and_bool(
    name_value: &MetaNameValue,
) -> syn::Result<IdentOrBool> {
    match &name_value.value {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Str(lit) => match lit.parse::<Ident>() {
                Ok(ident) => return Ok(IdentOrBool::Ident(ident)),
                Err(_) if lit.value().is_empty() => {
                    return Ok(IdentOrBool::Bool(false));
                },
                Err(error) => {
                    return Err(error);
                },
            },
            Lit::Bool(lit) => {
                return Ok(IdentOrBool::Bool(lit.value));
            },
            _ => (),
        },
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                return Ok(IdentOrBool::Ident(ident.clone()));
            }
        },
        _ => (),
    }

    Err(syn::Error::new(
        name_value.value.span(),
        format!(
            "expected `{path} = Ident` or `{path} = false`",
            path = path_to_string(&name_value.path)
        ),
    ))
}

#[inline]
pub(crate) fn meta_2_ident_and_bool(meta: &Meta) -> syn::Result<IdentOrBool> {
    match &meta {
        Meta::NameValue(name_value) => meta_name_value_2_ident_and_bool(name_value),
        Meta::List(list) => list.parse_args::<IdentOrBool>(),
        Meta::Path(path) => Err(syn::Error::new(
            path.span(),
            format!(
                "expected `{path} = Ident`, `{path}(Ident)`, `{path} = false`, or `{path}(false)`",
                path = path_to_string(path)
            ),
        )),
    }
}
