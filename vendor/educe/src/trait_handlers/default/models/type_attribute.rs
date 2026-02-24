use proc_macro2::Span;
use syn::{punctuated::Punctuated, spanned::Spanned, Attribute, Expr, Meta, Token};

use crate::{
    common::{
        bound::Bound,
        expr::{auto_adjust_expr, meta_2_expr},
        ident_bool::meta_2_bool_allow_path,
    },
    panic, Trait,
};

pub(crate) struct TypeAttribute {
    pub(crate) flag:       bool,
    pub(crate) new:        bool,
    pub(crate) expression: Option<Expr>,
    pub(crate) bound:      Bound,
    pub(crate) span:       Span,
}

#[derive(Debug)]
pub(crate) struct TypeAttributeBuilder {
    pub(crate) enable_flag:       bool,
    pub(crate) enable_new:        bool,
    pub(crate) enable_expression: bool,
    pub(crate) enable_bound:      bool,
}

impl TypeAttributeBuilder {
    pub(crate) fn build_from_default_meta(&self, meta: &Meta) -> syn::Result<TypeAttribute> {
        debug_assert!(meta.path().is_ident("Default"));

        let mut flag = false;
        let mut new = false;
        let mut expression = None;
        let mut bound = Bound::Auto;

        let correct_usage_for_default_attribute = {
            let mut usage = vec![];

            if self.enable_flag {
                usage.push(stringify!(#[educe(Default)]));
            }

            if self.enable_new {
                usage.push(stringify!(#[educe(Default(new))]));
            }

            if self.enable_expression {
                usage.push(stringify!(#[educe(Default(expression = expr))]));
            }

            if self.enable_bound {
                usage.push(stringify!(#[educe(Default(bound(where_predicates)))]));
                usage.push(stringify!(#[educe(Default(bound = false))]));
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
            Meta::NameValue(_) => {
                return Err(panic::attribute_incorrect_format(
                    meta.path().get_ident().unwrap(),
                    &correct_usage_for_default_attribute,
                ));
            },
            Meta::List(list) => {
                let result =
                    list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

                let mut new_is_set = false;
                let mut expression_is_set = false;
                let mut bound_is_set = false;

                let mut handler = |meta: Meta| -> syn::Result<bool> {
                    if let Some(ident) = meta.path().get_ident() {
                        match ident.to_string().as_str() {
                            "new" => {
                                if !self.enable_new {
                                    return Ok(false);
                                }

                                let v = meta_2_bool_allow_path(&meta)?;

                                if new_is_set {
                                    return Err(panic::parameter_reset(ident));
                                }

                                new_is_set = true;

                                new = v;

                                return Ok(true);
                            },
                            "expression" | "expr" => {
                                if !self.enable_expression {
                                    return Ok(false);
                                }

                                let v = meta_2_expr(&meta)?;

                                if expression_is_set {
                                    return Err(panic::parameter_reset(ident));
                                }

                                expression_is_set = true;

                                expression = Some(auto_adjust_expr(v, None));

                                return Ok(true);
                            },
                            "bound" => {
                                if !self.enable_bound {
                                    return Ok(false);
                                }

                                let v = Bound::from_meta(&meta)?;

                                if bound_is_set {
                                    return Err(panic::parameter_reset(ident));
                                }

                                bound_is_set = true;

                                bound = v;

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

        Ok(TypeAttribute {
            flag,
            new,
            expression,
            bound,
            span: meta.span(),
        })
    }

    pub(crate) fn build_from_attributes(
        &self,
        attributes: &[Attribute],
        traits: &[Trait],
    ) -> syn::Result<TypeAttribute> {
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

                            output = Some(self.build_from_default_meta(&meta)?);
                        }
                    }
                }
            }
        }

        Ok(output.unwrap_or(TypeAttribute {
            flag:       false,
            new:        false,
            expression: None,
            bound:      Bound::Auto,
            span:       Span::call_site(),
        }))
    }
}
