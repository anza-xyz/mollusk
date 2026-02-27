use quote::quote;
use syn::{Data, DeriveInput, Meta, Path, Type};

use super::{
    models::{FieldAttributeBuilder, TypeAttributeBuilder},
    TraitHandler,
};
use crate::{common::ident_index::IdentOrIndex, Trait};

pub(crate) struct HashStructHandler;

impl TraitHandler for HashStructHandler {
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
            .build_from_hash_meta(meta)?;

        let mut hash_types: Vec<&Type> = Vec::new();

        let mut hash_token_stream = proc_macro2::TokenStream::new();

        if let Data::Struct(data) = &ast.data {
            let built_in_hash: Path = syn::parse2(quote!(::core::hash::Hash::hash)).unwrap();

            for (index, field) in data.fields.iter().enumerate() {
                let field_attribute = FieldAttributeBuilder {
                    enable_ignore: true,
                    enable_method: true,
                }
                .build_from_attributes(&field.attrs, traits)?;

                if field_attribute.ignore {
                    continue;
                }

                let field_name = if let Some(ident) = field.ident.as_ref() {
                    IdentOrIndex::from(ident)
                } else {
                    IdentOrIndex::from(index)
                };

                let hash = field_attribute.method.as_ref().unwrap_or_else(|| {
                    hash_types.push(&field.ty);
                    &built_in_hash
                });

                hash_token_stream.extend(quote!( #hash(&self.#field_name, state); ));
            }
        }

        let ident = &ast.ident;

        let bound = type_attribute.bound.into_where_predicates_by_generic_parameters_check_types(
            &ast.generics.params,
            &syn::parse2(quote!(::core::hash::Hash)).unwrap(),
            &hash_types,
            &[],
        );

        let mut generics = ast.generics.clone();
        let where_clause = generics.make_where_clause();

        for where_predicate in bound {
            where_clause.predicates.push(where_predicate);
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        token_stream.extend(quote! {
            impl #impl_generics ::core::hash::Hash for #ident #ty_generics #where_clause {
                #[inline]
                fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                    #hash_token_stream
                }
            }
        });

        Ok(())
    }
}
