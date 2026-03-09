use std::collections::BTreeMap;

use quote::{format_ident, quote};
use syn::{spanned::Spanned, Data, DeriveInput, Field, Fields, Ident, Meta, Path, Type};

use super::{
    models::{FieldAttribute, FieldAttributeBuilder, TypeAttributeBuilder},
    TraitHandler,
};
use crate::{common::tools::DiscriminantType, Trait};

pub(crate) struct PartialOrdEnumHandler;

impl TraitHandler for PartialOrdEnumHandler {
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
        .build_from_partial_ord_meta(meta)?;

        let mut partial_ord_types: Vec<&Type> = Vec::new();

        let mut partial_cmp_token_stream = proc_macro2::TokenStream::new();

        let discriminant_type = DiscriminantType::from_ast(ast)?;

        let mut arms_token_stream = proc_macro2::TokenStream::new();

        let mut all_unit = true;

        if let Data::Enum(data) = &ast.data {
            for variant in data.variants.iter() {
                let _ = TypeAttributeBuilder {
                    enable_flag: false, enable_bound: false
                }
                .build_from_attributes(&variant.attrs, traits)?;

                let variant_ident = &variant.ident;

                let built_in_partial_cmp: Path =
                    syn::parse2(quote!(::core::cmp::PartialOrd::partial_cmp)).unwrap();

                match &variant.fields {
                    Fields::Unit => {
                        arms_token_stream.extend(quote! {
                            Self::#variant_ident => {
                                return Some(::core::cmp::Ordering::Equal);
                            }
                        });
                    },
                    Fields::Named(_) => {
                        all_unit = false;

                        let mut pattern_self_token_stream = proc_macro2::TokenStream::new();
                        let mut pattern_other_token_stream = proc_macro2::TokenStream::new();
                        let mut block_token_stream = proc_macro2::TokenStream::new();

                        let mut fields: BTreeMap<isize, (&Field, Ident, Ident, FieldAttribute)> =
                            BTreeMap::new();

                        for (index, field) in variant.fields.iter().enumerate() {
                            let field_attribute = FieldAttributeBuilder {
                                enable_ignore: true,
                                enable_method: true,
                                enable_rank:   true,
                                rank:          isize::MIN + index as isize,
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

                            let rank = field_attribute.rank;

                            if fields.contains_key(&rank) {
                                return Err(super::panic::reuse_a_rank(
                                    field_attribute.rank_span.unwrap_or_else(|| field.span()),
                                    rank,
                                ));
                            }

                            fields.insert(
                                rank,
                                (field, field_name_var_self, field_name_var_other, field_attribute),
                            );
                        }

                        for (field, field_name_var_self, field_name_var_other, field_attribute) in
                            fields.values()
                        {
                            let partial_cmp =
                                field_attribute.method.as_ref().unwrap_or_else(|| {
                                    partial_ord_types.push(&field.ty);

                                    &built_in_partial_cmp
                                });

                            block_token_stream.extend(quote! {
                                match #partial_cmp(#field_name_var_self, #field_name_var_other) {
                                    Some(::core::cmp::Ordering::Equal) => (),
                                    Some(::core::cmp::Ordering::Greater) => return Some(::core::cmp::Ordering::Greater),
                                    Some(::core::cmp::Ordering::Less) => return Some(::core::cmp::Ordering::Less),
                                    None => return None,
                                }
                            });
                        }

                        arms_token_stream.extend(quote! {
                            Self::#variant_ident { #pattern_self_token_stream } => {
                                if let Self::#variant_ident { #pattern_other_token_stream } = other {
                                    #block_token_stream
                                }
                            }
                        });
                    },
                    Fields::Unnamed(_) => {
                        all_unit = false;

                        let mut pattern_token_stream = proc_macro2::TokenStream::new();
                        let mut pattern2_token_stream = proc_macro2::TokenStream::new();
                        let mut block_token_stream = proc_macro2::TokenStream::new();

                        let mut fields: BTreeMap<isize, (&Field, Ident, Ident, FieldAttribute)> =
                            BTreeMap::new();

                        for (index, field) in variant.fields.iter().enumerate() {
                            let field_attribute = FieldAttributeBuilder {
                                enable_ignore: true,
                                enable_method: true,
                                enable_rank:   true,
                                rank:          isize::MIN + index as isize,
                            }
                            .build_from_attributes(&field.attrs, traits)?;

                            let field_name_var_self = format_ident!("_{}", index);

                            if field_attribute.ignore {
                                pattern_token_stream.extend(quote!(_,));
                                pattern2_token_stream.extend(quote!(_,));

                                continue;
                            }

                            let field_name_var_other = format_ident!("_{}", field_name_var_self);

                            pattern_token_stream.extend(quote!(#field_name_var_self,));
                            pattern2_token_stream.extend(quote!(#field_name_var_other,));

                            let rank = field_attribute.rank;

                            if fields.contains_key(&rank) {
                                return Err(super::panic::reuse_a_rank(
                                    field_attribute.rank_span.unwrap_or_else(|| field.span()),
                                    rank,
                                ));
                            }

                            fields.insert(
                                rank,
                                (field, field_name_var_self, field_name_var_other, field_attribute),
                            );
                        }

                        for (field, field_name, field_name2, field_attribute) in fields.values() {
                            let partial_cmp =
                                field_attribute.method.as_ref().unwrap_or_else(|| {
                                    partial_ord_types.push(&field.ty);

                                    &built_in_partial_cmp
                                });

                            block_token_stream.extend(quote! {
                                match #partial_cmp(#field_name, #field_name2) {
                                    Some(::core::cmp::Ordering::Equal) => (),
                                    Some(::core::cmp::Ordering::Greater) => return Some(::core::cmp::Ordering::Greater),
                                    Some(::core::cmp::Ordering::Less) => return Some(::core::cmp::Ordering::Less),
                                    None => return None,
                                }
                            });
                        }

                        arms_token_stream.extend(quote! {
                            Self::#variant_ident ( #pattern_token_stream ) => {
                                if let Self::#variant_ident ( #pattern2_token_stream ) = other {
                                    #block_token_stream
                                }
                            }
                        });
                    },
                }
            }
        }

        if arms_token_stream.is_empty() {
            partial_cmp_token_stream.extend(quote!(Some(::core::cmp::Ordering::Equal)));
        } else {
            let discriminant_cmp = quote! {
                unsafe {
                    ::core::cmp::Ord::cmp(&*<*const _>::from(self).cast::<#discriminant_type>(), &*<*const _>::from(other).cast::<#discriminant_type>())
                }
            };

            partial_cmp_token_stream.extend(if all_unit {
                quote! {
                    match #discriminant_cmp {
                        ::core::cmp::Ordering::Equal => Some(::core::cmp::Ordering::Equal),
                        ::core::cmp::Ordering::Greater => Some(::core::cmp::Ordering::Greater),
                        ::core::cmp::Ordering::Less => Some(::core::cmp::Ordering::Less),
                    }
                }
            } else {
                quote! {
                    match #discriminant_cmp {
                        ::core::cmp::Ordering::Equal => {
                            match self {
                                #arms_token_stream
                            }

                            Some(::core::cmp::Ordering::Equal)
                        },
                        ::core::cmp::Ordering::Greater => Some(::core::cmp::Ordering::Greater),
                        ::core::cmp::Ordering::Less => Some(::core::cmp::Ordering::Less),
                    }
                }
            });
        }

        let ident = &ast.ident;

        let bound = type_attribute.bound.into_where_predicates_by_generic_parameters_check_types(
            &ast.generics.params,
            &syn::parse2(quote!(::core::cmp::PartialOrd)).unwrap(),
            &partial_ord_types,
            &[quote! {::core::cmp::PartialEq}],
        );

        let mut generics = ast.generics.clone();
        let where_clause = generics.make_where_clause();

        for where_predicate in bound {
            where_clause.predicates.push(where_predicate);
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        token_stream.extend(quote! {
            impl #impl_generics ::core::cmp::PartialOrd for #ident #ty_generics #where_clause {
                #[inline]
                fn partial_cmp(&self, other: &Self) -> Option<::core::cmp::Ordering> {
                    #partial_cmp_token_stream
                }
            }
        });

        Ok(())
    }
}
