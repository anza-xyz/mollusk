use quote::quote;
use syn::{spanned::Spanned, Data, DeriveInput, Field, Meta, Type};

use super::{
    models::{FieldAttribute, FieldAttributeBuilder, TypeAttributeBuilder},
    TraitHandler,
};
use crate::Trait;

pub(crate) struct DefaultUnionHandler;

impl TraitHandler for DefaultUnionHandler {
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

        if let Data::Union(data) = &ast.data {
            if let Some(expression) = type_attribute.expression {
                for field in data.fields.named.iter() {
                    let _ = FieldAttributeBuilder {
                        enable_flag:       false,
                        enable_expression: false,
                    }
                    .build_from_attributes(&field.attrs, traits, &field.ty)?;
                }

                default_token_stream.extend(quote!(#expression));
            } else {
                let (field, field_attribute) =
                    {
                        let fields = &data.fields.named;

                        if fields.len() == 1 {
                            let field = &fields[0];

                            let field_attribute = FieldAttributeBuilder {
                                enable_flag:       true,
                                enable_expression: true,
                            }
                            .build_from_attributes(&field.attrs, traits, &field.ty)?;

                            (field, field_attribute)
                        } else {
                            let mut default_field: Option<(&Field, FieldAttribute)> = None;

                            for field in fields {
                                let field_attribute = FieldAttributeBuilder {
                                    enable_flag:       true,
                                    enable_expression: true,
                                }
                                .build_from_attributes(&field.attrs, traits, &field.ty)?;

                                if field_attribute.flag || field_attribute.expression.is_some() {
                                    if default_field.is_some() {
                                        return Err(super::panic::multiple_default_fields(
                                            field_attribute.span,
                                        ));
                                    }

                                    default_field = Some((field, field_attribute));
                                }
                            }

                            if let Some(default_field) = default_field {
                                default_field
                            } else {
                                return Err(super::panic::no_default_field(meta.span()));
                            }
                        }
                    };

                let mut fields_token_stream = proc_macro2::TokenStream::new();

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

                default_token_stream.extend(quote! {
                    Self {
                        #fields_token_stream
                    }
                });
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
