use syn::{punctuated::Punctuated, Attribute, Meta, Token};

use crate::{panic, supported_traits::Trait};

pub(crate) struct FieldAttribute;

pub(crate) struct FieldAttributeBuilder;

impl FieldAttributeBuilder {
    pub(crate) fn build_from_copy_meta(&self, meta: &Meta) -> syn::Result<FieldAttribute> {
        debug_assert!(meta.path().is_ident("Copy"));

        return Err(panic::attribute_incorrect_place(meta.path().get_ident().unwrap()));
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

        Ok(output.unwrap_or(FieldAttribute))
    }
}
