use std::collections::HashMap;

use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Field, Fields, Ident, Meta, Path, Type};

use super::{
    models::{FieldAttribute, FieldAttributeBuilder, TypeAttributeBuilder},
    TraitHandlerMultiple,
};
use crate::{panic, Trait};

pub(crate) struct IntoEnumHandler;

impl TraitHandlerMultiple for IntoEnumHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &[Meta],
    ) -> syn::Result<()> {
        let type_attribute = TypeAttributeBuilder {
            enable_types: true
        }
        .build_from_into_meta(meta)?;

        if let Data::Enum(data) = &ast.data {
            let field_attributes: Vec<HashMap<usize, FieldAttribute>> = {
                let mut map = Vec::new();

                for variant in data.variants.iter() {
                    let mut field_map = HashMap::new();

                    let _ = TypeAttributeBuilder {
                        enable_types: false
                    }
                    .build_from_attributes(&variant.attrs, traits)?;

                    for (index, field) in variant.fields.iter().enumerate() {
                        let field_attribute = FieldAttributeBuilder {
                            enable_types: true
                        }
                        .build_from_attributes(&field.attrs, traits)?;

                        for ty in field_attribute.types.keys() {
                            if !type_attribute.types.contains_key(ty) {
                                return Err(super::panic::no_into_impl(ty));
                            }
                        }

                        field_map.insert(index, field_attribute);
                    }

                    map.push(field_map);
                }

                map
            };

            for (target_ty, bound) in type_attribute.types {
                let mut into_types: Vec<&Type> = Vec::new();

                let mut arms_token_stream = proc_macro2::TokenStream::new();

                type Variants<'a> =
                    Vec<(&'a Ident, bool, usize, Ident, &'a Type, Option<&'a Path>)>;

                let mut variants: Variants = Vec::new();

                for (variant, field_attributes) in data.variants.iter().zip(field_attributes.iter())
                {
                    if let Fields::Unit = &variant.fields {
                        return Err(panic::trait_not_support_unit_variant(
                            meta[0].path().get_ident().unwrap(),
                            variant,
                        ));
                    }

                    let (index, field, method) = {
                        let fields = &variant.fields;

                        if fields.len() == 1 {
                            let field = fields.into_iter().next().unwrap();

                            let method = if let Some(field_attribute) = field_attributes.get(&0) {
                                if let Some(method) = field_attribute.types.get(&target_ty) {
                                    method.as_ref()
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            (0usize, field, method)
                        } else {
                            let mut into_field: Option<(usize, &Field, Option<&Path>)> = None;

                            for (index, field) in fields.iter().enumerate() {
                                if let Some(field_attribute) = field_attributes.get(&index) {
                                    if let Some((key, method)) =
                                        field_attribute.types.get_key_value(&target_ty)
                                    {
                                        if into_field.is_some() {
                                            return Err(super::panic::multiple_into_fields(key));
                                        }

                                        into_field = Some((index, field, method.as_ref()));
                                    }
                                }
                            }

                            if into_field.is_none() {
                                // search the same type
                                for (index, field) in fields.iter().enumerate() {
                                    let field_ty = super::common::to_hash_type(&field.ty);

                                    if target_ty.eq(&field_ty) {
                                        if into_field.is_some() {
                                            // multiple candidates
                                            into_field = None;

                                            break;
                                        }

                                        into_field = Some((index, field, None));
                                    }
                                }
                            }

                            if let Some(into_field) = into_field {
                                into_field
                            } else {
                                return Err(super::panic::no_into_field(&target_ty));
                            }
                        }
                    };

                    let (field_name, is_tuple): (Ident, bool) = match field.ident.as_ref() {
                        Some(ident) => (ident.clone(), false),
                        None => (format_ident!("_{}", index), true),
                    };

                    variants.push((&variant.ident, is_tuple, index, field_name, &field.ty, method));
                }

                if variants.is_empty() {
                    return Err(super::panic::no_into_field(&target_ty));
                }

                for (variant_ident, is_tuple, index, field_name, ty, method) in variants {
                    let mut pattern_token_stream = proc_macro2::TokenStream::new();
                    let mut body_token_stream = proc_macro2::TokenStream::new();

                    if let Some(method) = method {
                        body_token_stream.extend(quote!( #method(#field_name) ));
                    } else {
                        let field_ty = super::common::to_hash_type(ty);

                        if target_ty.eq(&field_ty) {
                            body_token_stream.extend(quote!( #field_name ));
                        } else {
                            into_types.push(ty);

                            body_token_stream
                                .extend(quote!( ::core::convert::Into::into(#field_name) ));
                        }
                    }

                    if is_tuple {
                        for _ in 0..index {
                            pattern_token_stream.extend(quote!(_,));
                        }

                        pattern_token_stream.extend(quote!( #field_name, .. ));

                        arms_token_stream.extend(
                            quote!( Self::#variant_ident ( #pattern_token_stream ) => #body_token_stream, ),
                        );
                    } else {
                        pattern_token_stream.extend(quote!( #field_name, .. ));

                        arms_token_stream.extend(
                            quote!( Self::#variant_ident { #pattern_token_stream } => #body_token_stream, ),
                        );
                    }
                }

                let ident = &ast.ident;

                let bound = bound.into_where_predicates_by_generic_parameters_check_types(
                    &ast.generics.params,
                    &syn::parse2(quote!(::core::convert::Into<#target_ty>)).unwrap(),
                    &into_types,
                    &[],
                );

                // clone generics in order to not to affect other Into<T> implementations
                let mut generics = ast.generics.clone();

                let where_clause = generics.make_where_clause();

                for where_predicate in bound {
                    where_clause.predicates.push(where_predicate);
                }

                let (impl_generics, ty_generics, _) = ast.generics.split_for_impl();

                token_stream.extend(quote! {
                    impl #impl_generics ::core::convert::Into<#target_ty> for #ident #ty_generics #where_clause {
                        #[inline]
                        fn into(self) -> #target_ty {
                            match self {
                                #arms_token_stream
                            }
                        }
                    }
                });
            }
        }

        Ok(())
    }
}
