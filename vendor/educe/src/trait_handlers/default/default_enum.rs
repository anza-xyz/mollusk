use quote::quote;
use syn::{spanned::Spanned, Data, DeriveInput, Fields, Meta, Type, Variant};

use super::{
    models::{FieldAttributeBuilder, TypeAttributeBuilder},
    TraitHandler,
};
use crate::Trait;

pub(crate) struct DefaultEnumHandler;

impl TraitHandler for DefaultEnumHandler {
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        let type_attribute = TypeAttributeBuilder {
            enable_flag:       true,
            enable_new:        true,
            enable_expression: true,
            enable_bound:      true,
        }
        .build_from_default_meta(meta)?;

        let mut default_types: Vec<&Type> = Vec::new();

        let mut default_token_stream = proc_macro2::TokenStream::new();

        if let Data::Enum(data) = &ast.data {
            if let Some(expression) = type_attribute.expression {
                for variant in data.variants.iter() {
                    let _ = TypeAttributeBuilder {
                        enable_flag:       false,
                        enable_new:        false,
                        enable_expression: false,
                        enable_bound:      false,
                    }
                    .build_from_attributes(&variant.attrs, traits)?;

                    ensure_fields_no_attribute(&variant.fields, traits)?;
                }

                default_token_stream.extend(quote!(#expression));
            } else {
                let variant = {
                    let variants = &data.variants;

                    if variants.len() == 1 {
                        let variant = &variants[0];

                        let _ = TypeAttributeBuilder {
                            enable_flag:       true,
                            enable_new:        false,
                            enable_expression: false,
                            enable_bound:      false,
                        }
                        .build_from_attributes(&variant.attrs, traits)?;

                        variant
                    } else {
                        let mut default_variant: Option<&Variant> = None;

                        for variant in variants {
                            let type_attribute = TypeAttributeBuilder {
                                enable_flag:       true,
                                enable_new:        false,
                                enable_expression: false,
                                enable_bound:      false,
                            }
                            .build_from_attributes(&variant.attrs, traits)?;

                            if type_attribute.flag {
                                if default_variant.is_some() {
                                    return Err(super::panic::multiple_default_variants(
                                        type_attribute.span,
                                    ));
                                }

                                default_variant = Some(variant);
                            } else {
                                ensure_fields_no_attribute(&variant.fields, traits)?;
                            }
                        }

                        if let Some(default_variant) = default_variant {
                            default_variant
                        } else {
                            return Err(super::panic::no_default_variant(meta.span()));
                        }
                    }
                };

                let variant_ident = &variant.ident;

                match &variant.fields {
                    Fields::Unit => {
                        default_token_stream.extend(quote!(Self::#variant_ident));
                    },
                    Fields::Named(_) => {
                        let mut fields_token_stream = proc_macro2::TokenStream::new();

                        for field in variant.fields.iter() {
                            let field_attribute = FieldAttributeBuilder {
                                enable_flag:       false,
                                enable_expression: true,
                            }
                            .build_from_attributes(&field.attrs, traits, &field.ty)?;

                            let field_name = field.ident.as_ref().unwrap();

                            if let Some(expression) = field_attribute.expression {
                                fields_token_stream.extend(quote! {
                                    #field_name: #expression,
                                });
                            } else {
                                let ty = &field.ty;

                                default_types.push(ty);

                                fields_token_stream.extend(quote! {
                                    #field_name: <#ty as ::core::default::Default>::default(),
                                });
                            }
                        }

                        default_token_stream.extend(quote! {
                            Self::#variant_ident {
                                #fields_token_stream
                            }
                        });
                    },
                    Fields::Unnamed(_) => {
                        let mut fields_token_stream = proc_macro2::TokenStream::new();

                        for field in variant.fields.iter() {
                            let field_attribute = FieldAttributeBuilder {
                                enable_flag:       false,
                                enable_expression: true,
                            }
                            .build_from_attributes(&field.attrs, traits, &field.ty)?;

                            if let Some(expression) = field_attribute.expression {
                                fields_token_stream.extend(quote!(#expression,));
                            } else {
                                let ty = &field.ty;

                                default_types.push(ty);

                                fields_token_stream
                                    .extend(quote!(<#ty as ::core::default::Default>::default(),));
                            }
                        }

                        default_token_stream
                            .extend(quote!(Self::#variant_ident ( #fields_token_stream )));
                    },
                }
            }
        }

        let ident = &ast.ident;

        let bound = type_attribute.bound.into_where_predicates_by_generic_parameters_check_types(
            &ast.generics.params,
            &syn::parse2(quote!(::core::default::Default)).unwrap(),
            &default_types,
            &[],
        );

        let mut generics = ast.generics.clone();
        let where_clause = generics.make_where_clause();

        for where_predicate in bound {
            where_clause.predicates.push(where_predicate);
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        token_stream.extend(quote! {
            impl #impl_generics ::core::default::Default for #ident #ty_generics #where_clause {
                #[inline]
                fn default() -> Self {
                    #default_token_stream
                }
            }
        });

        if type_attribute.new {
            token_stream.extend(quote! {
                impl #impl_generics #ident #ty_generics #where_clause {
                    /// Returns the "default value" for a type.
                    #[inline]
                    pub fn new() -> Self {
                        <Self as ::core::default::Default>::default()
                    }
                }
            });
        }

        Ok(())
    }
}

fn ensure_fields_no_attribute(fields: &Fields, traits: &[Trait]) -> syn::Result<()> {
    match fields {
        Fields::Unit => (),
        Fields::Named(fields) => {
            for field in fields.named.iter() {
                let _ = FieldAttributeBuilder {
                    enable_flag:       false,
                    enable_expression: false,
                }
                .build_from_attributes(&field.attrs, traits, &field.ty)?;
            }
        },
        Fields::Unnamed(fields) => {
            for field in fields.unnamed.iter() {
                let _ = FieldAttributeBuilder {
                    enable_flag:       false,
                    enable_expression: false,
                }
                .build_from_attributes(&field.attrs, traits, &field.ty)?;
            }
        },
    }

    Ok(())
}
