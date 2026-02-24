use quote::quote;
use syn::{Data, DeriveInput, Meta, Type};

use super::{
    models::{FieldAttributeBuilder, TypeAttributeBuilder},
    TraitHandler,
};
use crate::{common::ident_index::IdentOrIndex, Trait};

pub(crate) struct PartialEqStructHandler;

impl TraitHandler for PartialEqStructHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        let type_attribute =
            TypeAttributeBuilder {
                enable_flag: true, enable_unsafe: false, enable_bound: true
            }
            .build_from_partial_eq_meta(meta)?;

        let mut partial_eq_types: Vec<&Type> = Vec::new();

        let mut eq_token_stream = proc_macro2::TokenStream::new();

        if let Data::Struct(data) = &ast.data {
            for (index, field) in data.fields.iter().enumerate() {
                let field_attribute = FieldAttributeBuilder {
                    enable_ignore: true,
                    enable_method: true,
                }
                .build_from_attributes(&field.attrs, traits)?;

                if field_attribute.ignore {
                    continue;
                }

                let field_name = IdentOrIndex::from_ident_with_index(field.ident.as_ref(), index);

                if let Some(method) = field_attribute.method {
                    eq_token_stream.extend(quote! {
                        if !#method(&self.#field_name, &other.#field_name) {
                            return false;
                        }
                    });
                } else {
                    let ty = &field.ty;

                    partial_eq_types.push(ty);

                    eq_token_stream.extend(quote! {
                        if ::core::cmp::PartialEq::ne(&self.#field_name, &other.#field_name) {
                            return false;
                        }
                    });
                }
            }
        }

        let ident = &ast.ident;

        let bound = type_attribute.bound.into_where_predicates_by_generic_parameters_check_types(
            &ast.generics.params,
            &syn::parse2(quote!(::core::cmp::PartialEq)).unwrap(),
            &partial_eq_types,
            &[],
        );

        let mut generics = ast.generics.clone();
        let where_clause = generics.make_where_clause();

        for where_predicate in bound {
            where_clause.predicates.push(where_predicate);
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        token_stream.extend(quote! {
            impl #impl_generics ::core::cmp::PartialEq for #ident #ty_generics #where_clause {
                #[inline]
                fn eq(&self, other: &Self) -> bool {
                    #eq_token_stream

                    true
                }
            }
        });

        #[cfg(feature = "Eq")]
        if traits.contains(&Trait::Eq) {
            token_stream.extend(quote! {
                impl #impl_generics ::core::cmp::Eq for #ident #ty_generics #where_clause {
                }
            });
        }

        Ok(())
    }
}
