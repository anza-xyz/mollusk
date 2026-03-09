use syn::{punctuated::Punctuated, Attribute, Ident, Meta, Path, Token};

use crate::{
    common::{
        ident_bool::{
            meta_2_bool_allow_path, meta_2_ident, meta_name_value_2_bool, meta_name_value_2_ident,
            meta_name_value_2_ident_and_bool, IdentOrBool,
        },
        path::meta_2_path,
    },
    panic,
    supported_traits::Trait,
};

#[derive(Debug, Clone)]
pub(crate) enum FieldName {
    Default,
    Custom(Ident),
}

pub(crate) struct FieldAttribute {
    pub(crate) name:   FieldName,
    pub(crate) ignore: bool,
    pub(crate) method: Option<Path>,
}

pub(crate) struct FieldAttributeBuilder {
    pub(crate) enable_name:   bool,
    pub(crate) enable_ignore: bool,
    pub(crate) enable_method: bool,
    pub(crate) name:          FieldName,
}

impl FieldAttributeBuilder {
    pub(crate) fn build_from_debug_meta(&self, meta: &Meta) -> syn::Result<FieldAttribute> {
        debug_assert!(meta.path().is_ident("Debug"));

        let mut name = self.name.clone();
        let mut ignore = false;
        let mut method = None;

        let correct_usage_for_debug_attribute = {
            let mut usage = vec![];

            if self.enable_name {
                usage.push(stringify!(#[educe(Debug = NewName)]));
                usage.push(stringify!(#[educe(Debug(name(NewName)))]));
            }

            if self.enable_ignore {
                usage.push(stringify!(#[educe(Debug = false)]));
                usage.push(stringify!(#[educe(Debug(ignore))]));
            }

            if self.enable_method {
                usage.push(stringify!(#[educe(Debug(method(path_to_method)))]));
            }

            usage
        };

        match meta {
            Meta::Path(_) => {
                return Err(panic::attribute_incorrect_format(
                    meta.path().get_ident().unwrap(),
                    &correct_usage_for_debug_attribute,
                ));
            },
            Meta::NameValue(name_value) => {
                if self.enable_name {
                    if self.enable_ignore {
                        match meta_name_value_2_ident_and_bool(name_value)? {
                            IdentOrBool::Ident(ident) => {
                                name = FieldName::Custom(ident);
                            },
                            IdentOrBool::Bool(b) => {
                                ignore = !b;
                            },
                        }
                    } else {
                        name = FieldName::Custom(meta_name_value_2_ident(name_value)?);
                    }
                } else if self.enable_ignore {
                    ignore = !meta_name_value_2_bool(name_value)?;
                } else {
                    return Err(panic::attribute_incorrect_format(
                        meta.path().get_ident().unwrap(),
                        &correct_usage_for_debug_attribute,
                    ));
                }
            },
            Meta::List(list) => {
                let result =
                    list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

                let mut name_is_set = false;
                let mut ignore_is_set = false;
                let mut method_is_set = false;

                let mut handler = |meta: Meta| -> syn::Result<bool> {
                    if let Some(ident) = meta.path().get_ident() {
                        match ident.to_string().as_str() {
                            "name" | "rename" => {
                                if !self.enable_name {
                                    return Ok(false);
                                }

                                let v = meta_2_ident(&meta)?;

                                if name_is_set {
                                    return Err(panic::parameter_reset(ident));
                                }

                                name_is_set = true;

                                name = FieldName::Custom(v);

                                return Ok(true);
                            },
                            "ignore" => {
                                if !self.enable_ignore {
                                    return Ok(false);
                                }

                                let v = meta_2_bool_allow_path(&meta)?;

                                if ignore_is_set {
                                    return Err(panic::parameter_reset(ident));
                                }

                                ignore_is_set = true;

                                ignore = v;

                                return Ok(true);
                            },
                            "method" => {
                                if !self.enable_method {
                                    return Ok(false);
                                }

                                let v = meta_2_path(&meta)?;

                                if method_is_set {
                                    return Err(panic::parameter_reset(ident));
                                }

                                method_is_set = true;

                                method = Some(v);

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

        Ok(FieldAttribute {
            name,
            ignore,
            method,
        })
    }

    pub(crate) fn build_from_attributes(
        &self,
        attributes: &[Attribute],
        traits: &[Trait],
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

        Ok(output.unwrap_or(FieldAttribute {
            name:   self.name.clone(),
            ignore: false,
            method: None,
        }))
    }
}
