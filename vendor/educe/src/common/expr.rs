use quote::{quote, ToTokens};
use syn::{spanned::Spanned, Expr, Lit, Meta, Type};

use super::path::path_to_string;

const INT_TYPES: [&str; 12] =
    ["u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize"];

const FLOAT_TYPES: [&str; 2] = ["f32", "f64"];

#[inline]
pub(crate) fn meta_2_expr(meta: &Meta) -> syn::Result<Expr> {
    match &meta {
        Meta::NameValue(name_value) => Ok(name_value.value.clone()),
        Meta::List(list) => list.parse_args::<Expr>(),
        Meta::Path(path) => Err(syn::Error::new(
            path.span(),
            format!("expected `{path} = Expr` or `{path}(Expr)`", path = path_to_string(path)),
        )),
    }
}

#[inline]
pub(crate) fn auto_adjust_expr(expr: Expr, ty: Option<&Type>) -> Expr {
    match &expr {
        Expr::Lit(lit) => {
            match &lit.lit {
                Lit::Int(lit) => {
                    if let Some(Type::Path(ty)) = ty {
                        let ty_string = ty.into_token_stream().to_string();

                        if lit.suffix() == ty_string || INT_TYPES.contains(&ty_string.as_str()) {
                            // don't call into
                            return expr;
                        }
                    }
                },
                Lit::Float(lit) => {
                    if let Some(Type::Path(ty)) = ty {
                        let ty_string = ty.into_token_stream().to_string();

                        if lit.suffix() == ty_string || FLOAT_TYPES.contains(&ty_string.as_str()) {
                            // don't call into
                            return expr;
                        }
                    }
                },
                Lit::Str(_) => {
                    if let Some(Type::Reference(ty)) = ty {
                        let ty_string = ty.elem.clone().into_token_stream().to_string();

                        if ty_string == "str" {
                            // don't call into
                            return expr;
                        }
                    }
                },
                Lit::Bool(_) => {
                    if let Some(Type::Path(ty)) = ty {
                        let ty_string = ty.into_token_stream().to_string();

                        if ty_string == "bool" {
                            // don't call into
                            return expr;
                        }
                    }
                },
                Lit::Char(_) => {
                    if let Some(Type::Path(ty)) = ty {
                        let ty_string = ty.into_token_stream().to_string();

                        if ty_string == "char" {
                            // don't call into
                            return expr;
                        }
                    }
                },
                Lit::Byte(_) => {
                    if let Some(Type::Path(ty)) = ty {
                        let ty_string = ty.into_token_stream().to_string();

                        if ty_string == "u8" {
                            // don't call into
                            return expr;
                        }
                    }
                },
                Lit::ByteStr(_) => {
                    if let Some(Type::Reference(ty)) = ty {
                        if let Type::Array(ty) = ty.elem.as_ref() {
                            if let Type::Path(ty) = ty.elem.as_ref() {
                                let ty_string = ty.into_token_stream().to_string();

                                if ty_string == "u8" {
                                    // don't call into
                                    return expr;
                                }
                            }
                        }
                    }
                },
                _ => (),
            }

            syn::parse2(quote!(::core::convert::Into::into(#expr))).unwrap()
        },
        _ => expr,
    }
}
