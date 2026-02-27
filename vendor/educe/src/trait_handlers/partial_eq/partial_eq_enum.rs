use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Meta, Type};

use super::{
    models::{FieldAttributeBuilder, TypeAttributeBuilder},
    TraitHandler,
};
use crate::Trait;

pub(crate) struct PartialEqEnumHandler;

impl TraitHandler for PartialEqEnumHandler {
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

        let mut arms_token_stream = proc_macro2::TokenStream::new();

        if let Data::Enum(data) = &ast.data {
            for variant in data.variants.iter() {
                let _ = TypeAttributeBuilder {
                    enable_flag:   false,
                    enable_unsafe: false,
                    enable_bound:  false,
                }
                .build_from_attributes(&variant.attrs, traits)?;

                let variant_ident = &variant.ident;

                match &variant.fields {
                    Fields::Unit => {
                        arms_token_stream.extend(quote! {
                            Self::#variant_ident => {
                                if let Self::#variant_ident = other {
                                    // same
                                } else {
                                    return false;
                                }
                            }
                        });
                    },
                    Fields::Named(_) => {
                        let mut pattern_self_token_stream = proc_macro2::TokenStream::new();
                        let mut pattern_other_token_stream = proc_macro2::TokenStream::new();
                        let mut block_token_stream = proc_macro2::TokenStream::new();

                        for field in variant.fields.iter() {
                            let field_attribute = FieldAttributeBuilder {
                                enable_ignore: true,
                                enable_method: true,
                            }
                            .build_from_attributes(&field.attrs, traits)?;

                            let field_name_real = field.ident.as_ref().unwrap();
                            let field_name_var_self = format_ident!("_s_{}", field_name_real);
                            let field_name_var_other = format_ident!("_o_{}", field_name_real);

                            if field_attribute.ignore {
                                pattern_self_token_stream.extend(quote!(#field_name_real: _,));
                                pattern_other_token_stream.extend(quote!(#field_name_real: _,));

                                continue;
                            }

                            pattern_self_token_stream
                                .extend(quote!(#field_name_real: #field_name_var_self,));
                            pattern_other_token_stream
                                .extend(quote!(#field_name_real: #field_name_var_other,));

                            if let Some(method) = field_attribute.method {
                                block_token_stream.extend(quote! {
                                    if !#method(#field_name_var_self, #field_name_var_other) {
                                        return false;
                                    }
                                });
                            } else {
                                let ty = &field.ty;

                                partial_eq_types.push(ty);

                                block_token_stream.extend(quote! {
                                    if ::core::cmp::PartialEq::ne(#field_name_var_self, #field_name_var_other) {
                                        return false;
                                    }
                                });
                            }
                        }

                        arms_token_stream.extend(quote! {
                            Self::#variant_ident { #pattern_self_token_stream } => {
                                if let Self::#variant_ident { #pattern_other_token_stream } = other {
                                    #block_token_stream
                                } else {
                                    return false;
                                }
                            }
                        });
                    },
                    Fields::Unnamed(_) => {
                        let mut pattern_token_stream = proc_macro2::TokenStream::new();
                        let mut pattern2_token_stream = proc_macro2::TokenStream::new();
                        let mut block_token_stream = proc_macro2::TokenStream::new();

                        for (index, field) in variant.fields.iter().enumerate() {
                            let field_attribute = FieldAttributeBuilder {
                                enable_ignore: true,
                                enable_method: true,
                            }
                            .build_from_attributes(&field.attrs, traits)?;

                            if field_attribute.ignore {
                                pattern_token_stream.extend(quote!(_,));
                                pattern2_token_stream.extend(quote!(_,));

                                continue;
                            }

                            let field_name_var_self = format_ident!("_{}", index);

                            let field_name_var_other = format_ident!("_{}", field_name_var_self);

                            pattern_token_stream.extend(quote!(#field_name_var_self,));
                            pattern2_token_stream.extend(quote!(#field_name_var_other,));

                            if let Some(method) = field_attribute.method {
                                block_token_stream.extend(quote! {
                                    if !#method(#field_name_var_self, #field_name_var_other) {
                                        return false;
                                    }
                                });
                            } else {
                                let ty = &field.ty;

                                partial_eq_types.push(ty);

                                block_token_stream.extend(quote! {
                                    if ::core::cmp::PartialEq::ne(#field_name_var_self, #field_name_var_other) {
                                        return false;
                                    }
                                });
                            }
                        }

                        arms_token_stream.extend(quote! {
                            Self::#variant_ident ( #pattern_token_stream ) => {
                                if let Self::#variant_ident ( #pattern2_token_stream ) = other {
                                    #block_token_stream
                                } else {
                                    return false;
                                }
                            }
                        });
                    },
                }
            }
        }

        if !arms_token_stream.is_empty() {
            eq_token_stream.extend(quote! {
                match self {
                    #arms_token_stream
                }
            });
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
