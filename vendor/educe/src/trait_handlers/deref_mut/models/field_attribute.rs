use proc_macro2::Span;
use syn::{punctuated::Punctuated, spanned::Spanned, Attribute, Meta, Token};

use crate::{panic, supported_traits::Trait};

pub(crate) struct FieldAttribute {
    pub(crate) flag: bool,
    pub(crate) span: Span,
}

pub(crate) struct FieldAttributeBuilder {
    pub(crate) enable_flag: bool,
}

impl FieldAttributeBuilder {
    pub(crate) fn build_from_deref_mut_meta(&self, meta: &Meta) -> syn::Result<FieldAttribute> {
        debug_assert!(meta.path().is_ident("DerefMut"));

        let correct_usage_for_deref_mut_attribute = {
            let mut usage = vec![];

            if self.enable_flag {
                usage.push(stringify!(#[educe(DerefMut)]));
            }

            usage
        };

        match meta {
            Meta::Path(_) => {
                if !self.enable_flag {
                    return Err(panic::attribute_incorrect_format(
                        meta.path().get_ident().unwrap(),
                        &correct_usage_for_deref_mut_attribute,
                    ));
                }
            },
            Meta::NameValue(_) | Meta::List(_) => {
                return Err(panic::attribute_incorrect_format(
                    meta.path().get_ident().unwrap(),
                    &correct_usage_for_deref_mut_attribute,
                ));
            },
        }

        Ok(FieldAttribute {
            flag: true, span: meta.span()
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

                        if t == Trait::DerefMut {
                            if output.is_some() {
                                return Err(panic::reuse_a_trait(path.get_ident().unwrap()));
                            }

                            output = Some(self.build_from_deref_mut_meta(&meta)?);
                        }
                    }
                }
            }
        }

        Ok(output.unwrap_or(FieldAttribute {
            flag: false, span: Span::call_site()
        }))
    }
}
