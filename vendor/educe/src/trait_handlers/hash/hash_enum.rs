use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Meta, Path, Type};

use super::{
    models::{FieldAttributeBuilder, TypeAttributeBuilder},
    TraitHandler,
};
use crate::Trait;

pub(crate) struct HashEnumHandler;

impl TraitHandler for HashEnumHandler {
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

        let mut arms_token_stream = proc_macro2::TokenStream::new();

        if let Data::Enum(data) = &ast.data {
            for (variant_index, variant) in data.variants.iter().enumerate() {
                let _ = TypeAttributeBuilder {
                    enable_flag:   false,
                    enable_unsafe: false,
                    enable_bound:  false,
                }
                .build_from_attributes(&variant.attrs, traits)?;

                let variant_ident = &variant.ident;

                let built_in_hash: Path = syn::parse2(quote!(::core::hash::Hash::hash)).unwrap();

                match &variant.fields {
                    Fields::Unit => {
                        arms_token_stream.extend(quote! {
                            Self::#variant_ident => {
                                ::core::hash::Hash::hash(&#variant_index, state);
                            }
                        });
                    },
                    Fields::Named(_) => {
                        let mut pattern_token_stream = proc_macro2::TokenStream::new();
                        let mut block_token_stream = proc_macro2::TokenStream::new();

                        for field in variant.fields.iter() {
                            let field_attribute = FieldAttributeBuilder {
                                enable_ignore: true,
                                enable_method: true,
                            }
                            .build_from_attributes(&field.attrs, traits)?;

                            let field_name_real = field.ident.as_ref().unwrap();
                            let field_name_var = format_ident!("v_{}", field_name_real);

                            if field_attribute.ignore {
                                pattern_token_stream.extend(quote!(#field_name_real: _,));

                                continue;
                            }

                            pattern_token_stream.extend(quote!(#field_name_real: #field_name_var,));

                            let hash = field_attribute.method.as_ref().unwrap_or_else(|| {
                                hash_types.push(&field.ty);
                                &built_in_hash
                            });

                            block_token_stream.extend(quote!( #hash(#field_name_var, state); ));
                        }

                        arms_token_stream.extend(quote! {
                            Self::#variant_ident { #pattern_token_stream } => {
                                ::core::hash::Hash::hash(&#variant_index, state);

                                #block_token_stream
                            }
                        });
                    },
                    Fields::Unnamed(_) => {
                        let mut pattern_token_stream = proc_macro2::TokenStream::new();
                        let mut block_token_stream = proc_macro2::TokenStream::new();

                        for (index, field) in variant.fields.iter().enumerate() {
                            let field_attribute = FieldAttributeBuilder {
                                enable_ignore: true,
                                enable_method: true,
                            }
                            .build_from_attributes(&field.attrs, traits)?;

                            let field_name_var = format_ident!("_{}", index);

                            if field_attribute.ignore {
                                pattern_token_stream.extend(quote!(_,));

                                continue;
                            }

                            pattern_token_stream.extend(quote!(#field_name_var,));

                            let hash = field_attribute.method.as_ref().unwrap_or_else(|| {
                                hash_types.push(&field.ty);
                                &built_in_hash
                            });

                            block_token_stream.extend(quote!( #hash(#field_name_var, state); ));
                        }

                        arms_token_stream.extend(quote! {
                            Self::#variant_ident ( #pattern_token_stream ) => {
                                ::core::hash::Hash::hash(&#variant_index, state);

                                #block_token_stream
                            }
                        });
                    },
                }
            }
        }

        if !arms_token_stream.is_empty() {
            hash_token_stream.extend(quote! {
                match self {
                    #arms_token_stream
                }
            });
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
