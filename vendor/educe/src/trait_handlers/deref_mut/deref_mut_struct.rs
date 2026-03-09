use quote::quote;
use syn::{spanned::Spanned, Data, DeriveInput, Field, Meta, Type};

use super::{
    models::{FieldAttributeBuilder, TypeAttributeBuilder},
    TraitHandler,
};
use crate::{common::ident_index::IdentOrIndex, Trait};

pub(crate) struct DerefMutStructHandler;

impl TraitHandler for DerefMutStructHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        let _ = TypeAttributeBuilder {
            enable_flag: true
        }
        .build_from_deref_mut_meta(meta)?;

        let mut deref_mut_token_stream = proc_macro2::TokenStream::new();

        if let Data::Struct(data) = &ast.data {
            let (index, field) = {
                let fields = &data.fields;

                if fields.len() == 1 {
                    let field = fields.into_iter().next().unwrap();

                    let _ = FieldAttributeBuilder {
                        enable_flag: true
                    }
                    .build_from_attributes(&field.attrs, traits)?;

                    (0usize, field)
                } else {
                    let mut deref_field: Option<(usize, &Field)> = None;

                    for (index, field) in fields.iter().enumerate() {
                        let field_attribute = FieldAttributeBuilder {
                            enable_flag: true
                        }
                        .build_from_attributes(&field.attrs, traits)?;

                        if field_attribute.flag {
                            if deref_field.is_some() {
                                return Err(super::panic::multiple_deref_mut_fields(
                                    field_attribute.span,
                                ));
                            }

                            deref_field = Some((index, field));
                        }
                    }

                    if let Some(deref_field) = deref_field {
                        deref_field
                    } else {
                        return Err(super::panic::no_deref_mut_field(meta.span()));
                    }
                }
            };

            let field_name = IdentOrIndex::from_ident_with_index(field.ident.as_ref(), index);

            deref_mut_token_stream.extend(if let Type::Reference(_) = &field.ty {
                quote! (self.#field_name)
            } else {
                quote! (&mut self.#field_name)
            });
        }

        let ident = &ast.ident;

        let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

        token_stream.extend(quote! {
            impl #impl_generics ::core::ops::DerefMut for #ident #ty_generics #where_clause {
                #[inline]
                fn deref_mut(&mut self) -> &mut Self::Target {
                    #deref_mut_token_stream
                }
            }
        });

        Ok(())
    }
}
