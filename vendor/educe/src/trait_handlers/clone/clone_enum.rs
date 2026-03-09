use quote::{format_ident, quote};
use syn::{punctuated::Punctuated, Data, DeriveInput, Field, Fields, Meta, Type, Variant};

use super::models::{FieldAttribute, FieldAttributeBuilder, TypeAttributeBuilder};
use crate::{
    common::where_predicates_bool::WherePredicates, supported_traits::Trait, TraitHandler,
};

pub(crate) struct CloneEnumHandler;

impl TraitHandler for CloneEnumHandler {
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

        if let Data::Enum(data) = &ast.data {
            type Variants<'a> = Vec<(&'a Variant, Vec<(&'a Field, FieldAttribute)>)>;

            let mut variants: Variants = Vec::new();

            #[cfg(feature = "Copy")]
            let mut has_custom_clone_method = false;

            for variant in data.variants.iter() {
                let _ = TypeAttributeBuilder {
                    enable_flag: false, enable_bound: false
                }
                .build_from_attributes(&variant.attrs, traits)?;

                let mut variant_fields: Vec<(&Field, FieldAttribute)> = Vec::new();

                for field in variant.fields.iter() {
                    let field_attribute = FieldAttributeBuilder {
                        enable_method: true
                    }
                    .build_from_attributes(&field.attrs, traits)?;

                    #[cfg(feature = "Copy")]
                    if field_attribute.method.is_some() {
                        has_custom_clone_method = true;
                    }

                    variant_fields.push((field, field_attribute));
                }

                variants.push((variant, variant_fields));
            }

            #[cfg(feature = "Copy")]
            let contains_copy = !has_custom_clone_method && traits.contains(&Trait::Copy);

            #[cfg(not(feature = "Copy"))]
            let contains_copy = false;

            if contains_copy {
                clone_token_stream.extend(quote!(*self));
            }

            let mut clone_types: Vec<&Type> = Vec::new();

            if variants.is_empty() {
                if !contains_copy {
                    clone_token_stream.extend(quote!(unreachable!()));
                    clone_from_token_stream.extend(quote!(let _ = source;));
                }
            } else {
                let mut clone_variants_token_stream = proc_macro2::TokenStream::new();
                let mut clone_from_variants_token_stream = proc_macro2::TokenStream::new();

                for (variant, variant_fields) in variants {
                    let variant_ident = &variant.ident;

                    match &variant.fields {
                        Fields::Unit => {
                            clone_variants_token_stream.extend(quote! {
                                Self::#variant_ident => Self::#variant_ident,
                            });
                            clone_from_variants_token_stream.extend(quote! {
                                Self::#variant_ident => {
                                    if let Self::#variant_ident = source {
                                        // same
                                    } else {
                                        *self = ::core::clone::Clone::clone(source);
                                    }
                                },
                            });
                        },
                        Fields::Named(_) => {
                            let mut pattern_src_token_stream = proc_macro2::TokenStream::new();
                            let mut pattern_dst_token_stream = proc_macro2::TokenStream::new();
                            let mut cl_fields_token_stream = proc_macro2::TokenStream::new();
                            let mut cf_body_token_stream = proc_macro2::TokenStream::new();

                            for (field, field_attribute) in variant_fields {
                                let field_name_real = field.ident.as_ref().unwrap();
                                let field_name_src = format_ident!("_s_{}", field_name_real);
                                let field_name_dst = format_ident!("_d_{}", field_name_real);

                                pattern_src_token_stream
                                    .extend(quote!(#field_name_real: #field_name_src,));
                                pattern_dst_token_stream
                                    .extend(quote!(#field_name_real: #field_name_dst,));

                                if let Some(clone) = field_attribute.method.as_ref() {
                                    cl_fields_token_stream.extend(quote! {
                                        #field_name_real: #clone(#field_name_src),
                                    });
                                    cf_body_token_stream.extend(
                                        quote!(*#field_name_dst = #clone(#field_name_src);),
                                    );
                                } else {
                                    clone_types.push(&field.ty);

                                    cl_fields_token_stream.extend(quote! {
                                        #field_name_real: ::core::clone::Clone::clone(#field_name_src),
                                    });
                                    cf_body_token_stream.extend(
                                        quote!( ::core::clone::Clone::clone_from(#field_name_dst, #field_name_src); ),
                                    );
                                }
                            }

                            clone_variants_token_stream.extend(quote! {
                                    Self::#variant_ident { #pattern_src_token_stream } => Self::#variant_ident { #cl_fields_token_stream },
                                });

                            clone_from_variants_token_stream.extend(quote! {
                                    Self::#variant_ident { #pattern_dst_token_stream } => {
                                        if let Self::#variant_ident { #pattern_src_token_stream } = source {
                                            #cf_body_token_stream
                                        } else {
                                            *self = ::core::clone::Clone::clone(source);
                                        }
                                    },
                                });
                        },
                        Fields::Unnamed(_) => {
                            let mut pattern_token_stream = proc_macro2::TokenStream::new();
                            let mut pattern2_token_stream = proc_macro2::TokenStream::new();
                            let mut fields_token_stream = proc_macro2::TokenStream::new();
                            let mut body_token_stream = proc_macro2::TokenStream::new();

                            for (index, (field, field_attribute)) in
                                variant_fields.into_iter().enumerate()
                            {
                                let field_name_src = format_ident!("_{}", index);

                                pattern_token_stream.extend(quote!(#field_name_src,));

                                let field_name_dst = format_ident!("_{}", field_name_src);

                                pattern2_token_stream.extend(quote!(#field_name_dst,));

                                if let Some(clone) = field_attribute.method.as_ref() {
                                    fields_token_stream.extend(quote! (#clone(#field_name_src),));
                                    body_token_stream.extend(
                                        quote!(*#field_name_src = #clone(#field_name_dst);),
                                    );
                                } else {
                                    clone_types.push(&field.ty);

                                    fields_token_stream.extend(
                                        quote! ( ::core::clone::Clone::clone(#field_name_src), ),
                                    );
                                    body_token_stream.extend(
                                        quote!( ::core::clone::Clone::clone_from(#field_name_src, #field_name_dst); ),
                                    );
                                }
                            }

                            clone_variants_token_stream.extend(quote! {
                                    Self::#variant_ident ( #pattern_token_stream ) => Self::#variant_ident ( #fields_token_stream ),
                                });

                            clone_from_variants_token_stream.extend(quote! {
                                    Self::#variant_ident ( #pattern_token_stream ) => {
                                        if let Self::#variant_ident ( #pattern2_token_stream ) = source {
                                            #body_token_stream
                                        } else {
                                            *self = ::core::clone::Clone::clone(source);
                                        }
                                    },
                                });
                        },
                    }
                }

                if !contains_copy {
                    clone_token_stream.extend(quote! {
                        match self {
                            #clone_variants_token_stream
                        }
                    });

                    clone_from_token_stream.extend(quote! {
                        match self {
                            #clone_from_variants_token_stream
                        }
                    });
                }
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
