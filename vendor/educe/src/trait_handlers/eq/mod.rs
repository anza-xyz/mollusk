mod models;

use models::{FieldAttributeBuilder, TypeAttributeBuilder};
use quote::quote;
use syn::{Data, DeriveInput, Meta};

use super::TraitHandler;
use crate::Trait;

pub(crate) struct EqHandler;

impl TraitHandler for EqHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        #[cfg(feature = "PartialEq")]
        let contains_partial_eq = traits.contains(&Trait::PartialEq);

        #[cfg(not(feature = "PartialEq"))]
        let contains_partial_eq = false;

        let type_attribute = TypeAttributeBuilder {
            enable_flag:  true,
            enable_bound: !contains_partial_eq,
        }
        .build_from_eq_meta(meta)?;

        let mut field_types = vec![];

        // if `contains_partial_eq` is true, the implementation is handled by the `PartialEq` attribute, and field attributes is also handled by the `PartialEq` attribute
        if !contains_partial_eq {
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

            /*
                #[derive(PartialEq)]
                struct B<T> {
                    f1: PhantomData<T>,
                }

                impl<T> Eq for B<T> {

                }

                // The above code will throw a compile error because T have to be bound to `PartialEq`. However, it seems not to be necessary logically.
            */
            let bound =
                type_attribute.bound.into_where_predicates_by_generic_parameters_check_types(
                    &ast.generics.params,
                    &syn::parse2(quote!(::core::cmp::PartialEq)).unwrap(),
                    &field_types,
                    &[quote! {::core::cmp::PartialEq}],
                );

            let mut generics = ast.generics.clone();
            let where_clause = generics.make_where_clause();

            for where_predicate in bound {
                where_clause.predicates.push(where_predicate);
            }

            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

            token_stream.extend(quote! {
                impl #impl_generics ::core::cmp::Eq for #ident #ty_generics #where_clause {
                }
            });
        }

        Ok(())
    }
}
