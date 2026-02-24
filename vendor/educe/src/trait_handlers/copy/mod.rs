mod models;

use models::{FieldAttributeBuilder, TypeAttributeBuilder};
use quote::quote;
use syn::{Data, DeriveInput, Meta};

use super::TraitHandler;
use crate::Trait;

pub(crate) struct CopyHandler;

impl TraitHandler for CopyHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        #[cfg(feature = "Clone")]
        let contains_clone = traits.contains(&Trait::Clone);

        #[cfg(not(feature = "Clone"))]
        let contains_clone = false;

        let type_attribute = TypeAttributeBuilder {
            enable_flag:  true,
            enable_bound: !contains_clone,
        }
        .build_from_copy_meta(meta)?;

        let mut field_types = vec![];

        // if `contains_clone` is true, the implementation is handled by the `Clone` attribute, and field attributes is also handled by the `Clone` attribute
        if !contains_clone {
            match &ast.data {
                Data::Struct(data) => {
                    for field in data.fields.iter() {
                        field_types.push(&field.ty);
                        let _ =
                            FieldAttributeBuilder.build_from_attributes(&field.attrs, traits)?;
                    }
                },
                Data::Enum(data) => {
                    for variant in data.variants.iter() {
                        let _ = TypeAttributeBuilder {
                            enable_flag: false, enable_bound: false
                        }
                        .build_from_attributes(&variant.attrs, traits)?;

                        for field in variant.fields.iter() {
                            field_types.push(&field.ty);
                            let _ = FieldAttributeBuilder
                                .build_from_attributes(&field.attrs, traits)?;
                        }
                    }
                },
                Data::Union(data) => {
                    for field in data.fields.named.iter() {
                        field_types.push(&field.ty);
                        let _ =
                            FieldAttributeBuilder.build_from_attributes(&field.attrs, traits)?;
                    }
                },
            }

            let ident = &ast.ident;

            let bound =
                type_attribute.bound.into_where_predicates_by_generic_parameters_check_types(
                    &ast.generics.params,
                    &syn::parse2(quote!(::core::marker::Copy)).unwrap(),
                    &field_types,
                    &[quote! {::core::clone::Clone}],
                );

            let mut generics = ast.generics.clone();
            let where_clause = generics.make_where_clause();

            for where_predicate in bound {
                where_clause.predicates.push(where_predicate);
            }

            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

            token_stream.extend(quote! {
                impl #impl_generics ::core::marker::Copy for #ident #ty_generics #where_clause {
                }
            });
        }

        Ok(())
    }
}
