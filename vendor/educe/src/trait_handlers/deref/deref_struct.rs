use quote::quote;
use syn::{spanned::Spanned, Data, DeriveInput, Field, Meta};

use super::{
    models::{FieldAttributeBuilder, TypeAttributeBuilder},
    TraitHandler,
};
use crate::{
    common::{ident_index::IdentOrIndex, r#type::dereference_changed},
    Trait,
};

pub(crate) struct DerefStructHandler;

impl TraitHandler for DerefStructHandler {
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
        .build_from_deref_meta(meta)?;

        let mut target_token_stream = proc_macro2::TokenStream::new();
        let mut deref_token_stream = proc_macro2::TokenStream::new();

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
                                return Err(super::panic::multiple_deref_fields(
                                    field_attribute.span,
                                ));
                            }

                            deref_field = Some((index, field));
                        }
                    }

                    if let Some(deref_field) = deref_field {
                        deref_field
                    } else {
                        return Err(super::panic::no_deref_field(meta.span()));
                    }
                }
            };

            let ty = &field.ty;
            let (dereference_ty, is_ref) = dereference_changed(ty);

            target_token_stream.extend(quote!(#dereference_ty));

            let field_name = IdentOrIndex::from_ident_with_index(field.ident.as_ref(), index);

            deref_token_stream.extend(if is_ref {
                quote! (self.#field_name)
            } else {
                quote! (&self.#field_name)
            });
        }

        let ident = &ast.ident;

        let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

        token_stream.extend(quote! {
            impl #impl_generics ::core::ops::Deref for #ident #ty_generics #where_clause {
                type Target = #target_token_stream;

                #[inline]
                fn deref(&self) -> &Self::Target {
                    #deref_token_stream
                }
            }
        });

        Ok(())
    }
}
