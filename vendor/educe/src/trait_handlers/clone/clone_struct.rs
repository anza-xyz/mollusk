use quote::quote;
use syn::{punctuated::Punctuated, Data, DeriveInput, Field, Fields, Index, Meta, Type};

use super::models::{FieldAttribute, FieldAttributeBuilder, TypeAttributeBuilder};
use crate::{
    common::where_predicates_bool::WherePredicates, supported_traits::Trait, TraitHandler,
};

pub(crate) struct CloneStructHandler;

impl TraitHandler for CloneStructHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        let type_attribute = TypeAttributeBuilder {
            enable_flag: true, enable_bound: true
        }
        .build_from_clone_meta(meta)?;

        let mut bound: WherePredicates = Punctuated::new();

        let mut clone_token_stream = proc_macro2::TokenStream::new();
        let mut clone_from_token_stream = proc_macro2::TokenStream::new();

        if let Data::Struct(data) = &ast.data {
            let mut fields: Vec<(&Field, FieldAttribute)> = Vec::new();

            #[cfg(feature = "Copy")]
            let contains_copy = traits.contains(&Trait::Copy);

            #[cfg(not(feature = "Copy"))]
            let contains_copy = false;

            if contains_copy {
                clone_token_stream.extend(quote!(*self));
            }

            for field in data.fields.iter() {
                let field_attribute = FieldAttributeBuilder {
                    enable_method: !contains_copy
                }
                .build_from_attributes(&field.attrs, traits)?;

                fields.push((field, field_attribute));
            }

            let mut clone_types: Vec<&Type> = Vec::new();

            match &data.fields {
                Fields::Unit => {
                    if !contains_copy {
                        clone_token_stream.extend(quote!(Self));
                        clone_from_token_stream.extend(quote!(let _ = source;));
                    }
                },
                Fields::Named(_) => {
                    let mut fields_token_stream = proc_macro2::TokenStream::new();
                    let mut clone_from_body_token_stream = proc_macro2::TokenStream::new();

                    if fields.is_empty() {
                        clone_from_body_token_stream.extend(quote!(let _ = source;));
                    } else {
                        for (field, field_attribute) in fields {
                            let field_name = field.ident.as_ref().unwrap();

                            if let Some(clone) = field_attribute.method.as_ref() {
                                fields_token_stream.extend(quote! {
                                    #field_name: #clone(&self.#field_name),
                                });

                                clone_from_body_token_stream.extend(
                                    quote!(self.#field_name = #clone(&source.#field_name);),
                                );
                            } else {
                                clone_types.push(&field.ty);

                                fields_token_stream.extend(quote! {
                                    #field_name: ::core::clone::Clone::clone(&self.#field_name),
                                });

                                clone_from_body_token_stream.extend(
                                        quote!( ::core::clone::Clone::clone_from(&mut self.#field_name, &source.#field_name); ),
                                    );
                            }
                        }
                    }

                    if !contains_copy {
                        clone_token_stream.extend(quote! {
                            Self {
                                #fields_token_stream
                            }
                        });

                        clone_from_token_stream.extend(clone_from_body_token_stream);
                    }
                },
                Fields::Unnamed(_) => {
                    let mut fields_token_stream = proc_macro2::TokenStream::new();
                    let mut clone_from_body_token_stream = proc_macro2::TokenStream::new();

                    if fields.is_empty() {
                        clone_from_body_token_stream.extend(quote!(let _ = source;));
                    } else {
                        for (index, (field, field_attribute)) in fields.into_iter().enumerate() {
                            let field_name = Index::from(index);

                            if let Some(clone) = field_attribute.method.as_ref() {
                                fields_token_stream.extend(quote!(#clone(&self.#field_name),));

                                clone_from_body_token_stream.extend(
                                    quote!(self.#field_name = #clone(&source.#field_name);),
                                );
                            } else {
                                clone_types.push(&field.ty);

                                fields_token_stream.extend(
                                    quote! ( ::core::clone::Clone::clone(&self.#field_name), ),
                                );

                                clone_from_body_token_stream.extend(
                                        quote!( ::core::clone::Clone::clone_from(&mut self.#field_name, &source.#field_name); ),
                                    );
                            }
                        }
                    }

                    if !contains_copy {
                        clone_token_stream.extend(quote!(Self ( #fields_token_stream )));
                        clone_from_token_stream.extend(clone_from_body_token_stream);
                    }
                },
            }

            bound = type_attribute.bound.into_where_predicates_by_generic_parameters_check_types(
                &ast.generics.params,
                &syn::parse2(if contains_copy {
                    quote!(::core::marker::Copy)
                } else {
                    quote!(::core::clone::Clone)
                })
                .unwrap(),
                &clone_types,
                &[],
            );
        }

        let clone_from_fn_token_stream = if clone_from_token_stream.is_empty() {
            None
        } else {
            Some(quote! {
                #[inline]
                fn clone_from(&mut self, source: &Self) {
                    #clone_from_token_stream
                }
            })
        };

        let ident = &ast.ident;

        let mut generics = ast.generics.clone();
        let where_clause = generics.make_where_clause();

        for where_predicate in bound {
            where_clause.predicates.push(where_predicate);
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        token_stream.extend(quote! {
            impl #impl_generics ::core::clone::Clone for #ident #ty_generics #where_clause {
                #[inline]
                fn clone(&self) -> Self {
                    #clone_token_stream
                }

                #clone_from_fn_token_stream
            }
        });

        #[cfg(feature = "Copy")]
        if traits.contains(&Trait::Copy) {
            token_stream.extend(quote! {
                impl #impl_generics ::core::marker::Copy for #ident #ty_generics #where_clause {
                }
            });
        }

        Ok(())
    }
}
