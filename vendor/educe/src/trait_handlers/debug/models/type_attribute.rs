use proc_macro2::Ident;
use syn::{punctuated::Punctuated, Attribute, Meta, Token};

use crate::{
    common::{
        bound::Bound,
        ident_bool::{meta_2_bool, meta_2_ident_and_bool, meta_name_value_2_ident, IdentOrBool},
        unsafe_punctuated_meta::UnsafePunctuatedMeta,
    },
    panic, Trait,
};

#[derive(Debug, Clone)]
pub(crate) enum TypeName {
    Disable,
    Default,
    Custom(Ident),
}

impl TypeName {
    #[inline]
    pub(crate) fn to_ident_by_ident<'a, 'b: 'a>(&'a self, ident: &'b Ident) -> Option<&'a Ident> {
        match self {
            Self::Disable => None,
            Self::Default => Some(ident),
            Self::Custom(ident) => Some(ident),
        }
    }
}

pub(crate) struct TypeAttribute {
    pub(crate) has_unsafe:  bool,
    pub(crate) name:        TypeName,
    pub(crate) named_field: bool,
    pub(crate) bound:       Bound,
}

#[derive(Debug)]
pub(crate) struct TypeAttributeBuilder {
    pub(crate) enable_flag:        bool,
    pub(crate) enable_unsafe:      bool,
    pub(crate) enable_name:        bool,
    pub(crate) enable_named_field: bool,
    pub(crate) enable_bound:       bool,
    pub(crate) name:               TypeName,
    pub(crate) named_field:        bool,
}

impl TypeAttributeBuilder {
    pub(crate) fn build_from_debug_meta(&self, meta: &Meta) -> syn::Result<TypeAttribute> {
        debug_assert!(meta.path().is_ident("Debug"));

        let mut has_unsafe = false;
        let mut name = self.name.clone();
        let mut named_field = self.named_field;
        let mut bound = Bound::Auto;

        let correct_usage_for_debug_attribute = {
            let mut usage = vec![];

            if self.enable_flag {
                usage.push(stringify!(#[educe(Debug)]));
            }

            if self.enable_name {
                if !self.enable_unsafe {
                    usage.push(stringify!(#[educe(Debug = NewName)]));
                }

                usage.push(stringify!(#[educe(Debug(name(NewName)))]));

                if let TypeName::Disable = &name {
                    usage.push(stringify!(#[educe(Debug(name = true))]));
                } else {
                    usage.push(stringify!(#[educe(Debug(name = false))]));
                }
            }

            if self.enable_named_field {
                if !self.named_field {
                    usage.push(stringify!(#[educe(Debug(named_field = true))]));
                } else {
                    usage.push(stringify!(#[educe(Debug(named_field = false))]));
                }
            }

            if self.enable_bound {
                usage.push(stringify!(#[educe(Debug(bound(where_predicates)))]));
                usage.push(stringify!(#[educe(Debug(bound = false))]));
            }

            usage
        };

        match meta {
            Meta::Path(_) => {
                if !self.enable_flag {
                    return Err(panic::attribute_incorrect_format(
                        meta.path().get_ident().unwrap(),
                        &correct_usage_for_debug_attribute,
                    ));
                }
            },
            Meta::NameValue(name_value) => {
                if !self.enable_name {
                    return Err(panic::attribute_incorrect_format(
                        meta.path().get_ident().unwrap(),
                        &correct_usage_for_debug_attribute,
                    ));
                }

                name = TypeName::Custom(meta_name_value_2_ident(name_value)?);
            },
            Meta::List(list) => {
                let result = if self.enable_unsafe {
                    let result: UnsafePunctuatedMeta = list.parse_args()?;

                    has_unsafe = result.has_unsafe;

                    result.list
                } else {
                    list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?
                };

                let mut name_is_set = false;
                let mut named_field_is_set = false;
                let mut bound_is_set = false;

                let mut handler = |meta: Meta| -> syn::Result<bool> {
                    if let Some(ident) = meta.path().get_ident() {
                        match ident.to_string().as_str() {
                            "name" | "rename" => {
                                if !self.enable_name {
                                    return Ok(false);
                                }

                                let v = meta_2_ident_and_bool(&meta)?;

                                if name_is_set {
                                    return Err(panic::parameter_reset(ident));
                                }

                                name_is_set = true;

                                name = match v {
                                    IdentOrBool::Ident(ident) => TypeName::Custom(ident),
                                    IdentOrBool::Bool(b) => {
                                        if b {
                                            TypeName::Default
                                        } else {
                                            TypeName::Disable
                                        }
                                    },
                                };

                                return Ok(true);
                            },
                            "named_field" => {
                                if !self.enable_named_field {
                                    return Ok(false);
                                }

                                let v = meta_2_bool(&meta)?;

                                if named_field_is_set {
                                    return Err(panic::parameter_reset(ident));
                                }

                                named_field_is_set = true;

                                named_field = v;

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
                            &correct_usage_for_debug_attribute,
                        ));
                    }
                }
            },
        }

        Ok(TypeAttribute {
            has_unsafe,
            name,
            named_field,
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

                        if t == Trait::Debug {
                            if output.is_some() {
                                return Err(panic::reuse_a_trait(path.get_ident().unwrap()));
                            }

                            output = Some(self.build_from_debug_meta(&meta)?);
                        }
                    }
                }
            }
        }

        Ok(output.unwrap_or(TypeAttribute {
            has_unsafe:  false,
            name:        self.name.clone(),
            named_field: self.named_field,
            bound:       Bound::Auto,
        }))
    }
}
