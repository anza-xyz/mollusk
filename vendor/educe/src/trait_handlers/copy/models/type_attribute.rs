use syn::{punctuated::Punctuated, Attribute, Meta, Token};

use crate::{common::bound::Bound, panic, Trait};

pub(crate) struct TypeAttribute {
    pub(crate) bound: Bound,
}

#[derive(Debug)]
pub(crate) struct TypeAttributeBuilder {
    pub(crate) enable_flag:  bool,
    pub(crate) enable_bound: bool,
}

impl TypeAttributeBuilder {
    pub(crate) fn build_from_copy_meta(&self, meta: &Meta) -> syn::Result<TypeAttribute> {
        debug_assert!(meta.path().is_ident("Copy"));

        let mut bound = Bound::Auto;

        let correct_usage_for_copy_attribute = {
            let mut usage = vec![];

            if self.enable_flag {
                usage.push(stringify!(#[educe(Copy)]));
            }

            if self.enable_bound {
                usage.push(stringify!(#[educe(Copy(bound(where_predicates)))]));
                usage.push(stringify!(#[educe(Copy(bound = false))]));
            }

            usage
        };

        match meta {
            Meta::Path(_) => {
                if !self.enable_flag {
                    return Err(panic::attribute_incorrect_format(
                        meta.path().get_ident().unwrap(),
                        &correct_usage_for_copy_attribute,
                    ));
                }
            },
            Meta::NameValue(_) => {
                return Err(panic::attribute_incorrect_format(
                    meta.path().get_ident().unwrap(),
                    &correct_usage_for_copy_attribute,
                ));
            },
            Meta::List(list) => {
                let result =
                    list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

                let mut bound_is_set = false;

                let mut handler = |meta: Meta| -> syn::Result<bool> {
                    if let Some(ident) = meta.path().get_ident() {
                        if ident == "bound" {
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
                        }
                    }

                    Ok(false)
                };

                for p in result {
                    if !handler(p)? {
                        return Err(panic::attribute_incorrect_format(
                            meta.path().get_ident().unwrap(),
                            &correct_usage_for_copy_attribute,
                        ));
                    }
                }
            },
        }

        Ok(TypeAttribute {
            bound,
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

                        if t == Trait::Copy {
                            if output.is_some() {
                                return Err(panic::reuse_a_trait(path.get_ident().unwrap()));
                            }

                            output = Some(self.build_from_copy_meta(&meta)?);
                        }
                    }
                }
            }
        }

        Ok(output.unwrap_or(TypeAttribute {
            bound: Bound::Auto
        }))
    }
}
