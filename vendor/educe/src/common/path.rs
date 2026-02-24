use quote::ToTokens;
use syn::{spanned::Spanned, Expr, Lit, LitStr, Meta, MetaNameValue, Path};

#[inline]
pub(crate) fn meta_name_value_2_path(name_value: &MetaNameValue) -> syn::Result<Path> {
    match &name_value.value {
        Expr::Lit(lit) => {
            if let Lit::Str(lit) = &lit.lit {
                return lit.parse();
            }
        },
        Expr::Path(path) => return Ok(path.path.clone()),
        _ => (),
    }

    Err(syn::Error::new(
        name_value.value.span(),
        format!("expected `{path} = Path`", path = path_to_string(&name_value.path)),
    ))
}

#[inline]
pub(crate) fn meta_2_path(meta: &Meta) -> syn::Result<Path> {
    match &meta {
        Meta::NameValue(name_value) => meta_name_value_2_path(name_value),
        Meta::List(list) => {
            if let Ok(lit) = list.parse_args::<LitStr>() {
                lit.parse()
            } else {
                list.parse_args()
            }
        },
        Meta::Path(path) => Err(syn::Error::new(
            path.span(),
            format!("expected `{path} = Path` or `{path}(Path)`", path = path_to_string(path)),
        )),
    }
}

#[inline]
pub(crate) fn path_to_string(path: &Path) -> String {
    path.into_token_stream().to_string().replace(' ', "")
}
