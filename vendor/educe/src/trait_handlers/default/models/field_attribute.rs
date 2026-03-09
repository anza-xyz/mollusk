use proc_macro2::Span;
use syn::{punctuated::Punctuated, spanned::Spanned, Attribute, Expr, Meta, Token, Type};

use crate::{
    common::expr::{auto_adjust_expr, meta_2_expr},
    panic,
    supported_traits::Trait,
};

pub(crate) struct FieldAttribute {
    pub(crate) flag:       bool,
    pub(crate) expression: Option<Expr>,
    pub(crate) span:       Span,
}

pub(crate) struct FieldAttributeBuilder {
    pub(crate) enable_flag:       bool,
    pub(crate) enable_expression: bool,
}

impl FieldAttributeBuilder {
    pub(crate) fn build_from_default_meta(
        &self,
        meta: &Meta,
        ty: &Type,
    ) -> syn::Result<FieldAttribute> {
        debug_assert!(meta.path().is_ident("Default"));

        let mut flag = false;
        let mut expression = None;

        let correct_usage_for_default_attribute = {
            let mut usage = vec![];

            if self.enable_flag {
                usage.push(stringify!(#[educe(Default)]));
            }

            if self.enable_expression {
                usage.push(stringify!(#[educe(Default = expr)]));

                usage.push(stringify!(#[educe(Default(expression = expr))]));
            }

            usage
        };

        match meta {
            Meta::Path(_) => {
                if !self.enable_flag {
                    return Err(panic::attribute_incorrect_format(
                        meta.path().get_ident().unwrap(),
                        &correct_usage_for_default_attribute,
                    ));
                }

                flag = true;
            },
            Meta::NameValue(name_value) => {
                if !self.enable_expression {
                    return Err(panic::attribute_incorrect_format(
                        meta.path().get_ident().unwrap(),
                        &correct_usage_for_default_attribute,
                    ));
                }

                expression = Some(auto_adjust_expr(name_value.value.clone(), Some(ty)));
            },
            Meta::List(list) => {
                let result =
                    list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

                let mut expression_is_set = false;

                let mut handler = |meta: Meta| -> syn::Result<bool> {
                    if let Some(ident) = meta.path().get_ident() {
                        match ident.to_string().as_str() {
                            "expression" | "expr" => {
                                if !self.enable_expression {
                                    return Ok(false);
                                }

                                let v = meta_2_expr(&meta)?;

                                if expression_is_set {
                                    return Err(panic::parameter_reset(ident));
                                }

                                expression_is_set = true;

                                expression = Some(auto_adjust_expr(v, Some(ty)));

                                return Ok(true);
                            },
                            _ => (),
                        }
                    }

                    Ok(false)
                };

                for p in result {
                    if !handler(p)? {
                        return Err(panic::attribute_incorrect_format(
                            meta.path().get_ident().unwrap(),
                            &correct_usage_for_default_attribute,
                        ));
                    }
                }
            },
        }

        Ok(FieldAttribute {
            flag,
            expression,
            span: meta.span(),
        })
    }

    pub(crate) fn build_from_attributes(
        &self,
        attributes: &[Attribute],
        traits: &[Trait],
        ty: &Type,
    ) -> syn::Result<FieldAttribute> {
        let mut output = None;

        for attribute in attributes.iter() {
            let path = attribute.path();

            if path.is_ident("educe") {
                if let Meta::List(list) = &attribute.meta {
                    let result =
                        list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

                    for meta in result {
                        let path = meta.path();

                        let t = match Trait::from_path(path) {
                            Some(t) => t,
                            None => return Err(panic::unsupported_trait(meta.path())),
                        };

                        if !traits.contains(&t) {
                            return Err(panic::trait_not_used(path.get_ident().unwrap()));
                        }

                        if t == Trait::Default {
                            if output.is_some() {
                                return Err(panic::reuse_a_trait(path.get_ident().unwrap()));
                            }

                            output = Some(self.build_from_default_meta(&meta, ty)?);
                        }
                    }
                }
            }
        }

        Ok(output.unwrap_or(FieldAttribute {
            flag:       false,
            expression: None,
            span:       Span::call_site(),
        }))
    }
}
