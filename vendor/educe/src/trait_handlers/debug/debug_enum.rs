use quote::{format_ident, quote, ToTokens};
use syn::{Data, DeriveInput, Fields, Meta, Type};

use super::models::{FieldAttributeBuilder, FieldName, TypeAttributeBuilder, TypeName};
use crate::{common::path::path_to_string, supported_traits::Trait, trait_handlers::TraitHandler};

pub(crate) struct DebugEnumHandler;

impl TraitHandler for DebugEnumHandler {
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        let type_attribute = TypeAttributeBuilder {
            enable_flag:        true,
            enable_unsafe:      false,
            enable_name:        true,
            enable_named_field: false,
            enable_bound:       true,
            name:               TypeName::Disable,
            named_field:        false,
        }
        .build_from_debug_meta(meta)?;

        let name = type_attribute.name.to_ident_by_ident(&ast.ident);

        let mut debug_types: Vec<&Type> = Vec::new();

        let mut builder_token_stream = proc_macro2::TokenStream::new();

        let mut arms_token_stream = proc_macro2::TokenStream::new();

        if let Data::Enum(data) = &ast.data {
            for variant in data.variants.iter() {
                let type_attribute = TypeAttributeBuilder {
                    enable_flag:        false,
                    enable_unsafe:      false,
                    enable_name:        true,
                    enable_named_field: true,
                    enable_bound:       false,
                    name:               TypeName::Default,
                    named_field:        matches!(&variant.fields, Fields::Named(_)),
                }
                .build_from_attributes(&variant.attrs, traits)?;

                let variant_ident = &variant.ident;

                let variant_name = type_attribute.name.to_ident_by_ident(variant_ident);

                let named_field = type_attribute.named_field;

                let name_string = if let Some(name) = name {
                    if let Some(variant_name) = variant_name {
                        Some(path_to_string(&syn::parse2(quote!(#name::#variant_name)).unwrap()))
                    } else {
                        Some(name.into_token_stream().to_string())
                    }
                } else {
                    variant_name.map(|variant_name| variant_name.into_token_stream().to_string())
                };

                match &variant.fields {
                    Fields::Unit => {
                        if name_string.is_none() {
                            return Err(super::panic::unit_variant_need_name(variant));
                        }

                        arms_token_stream
                            .extend(quote!( Self::#variant_ident => f.write_str(#name_string), ));
                    },
                    Fields::Named(fields) => {
                        let mut has_fields = false;

                        let mut pattern_token_stream = proc_macro2::TokenStream::new();
                        let mut block_token_stream = proc_macro2::TokenStream::new();

                        if named_field {
                            block_token_stream
                                .extend(create_named_field_builder(name_string.as_deref()));

                            for field in fields.named.iter() {
                                let field_attribute = FieldAttributeBuilder {
                                    enable_name:   true,
                                    enable_ignore: true,
                                    enable_method: true,
                                    name:          FieldName::Default,
                                }
                                .build_from_attributes(&field.attrs, traits)?;

                                let field_name_real = field.ident.as_ref().unwrap();
                                let field_name_var = format_ident!("_{}", field_name_real);

                                if field_attribute.ignore {
                                    pattern_token_stream.extend(quote!(#field_name_real: _,));

                                    continue;
                                }

                                let key = match field_attribute.name {
                                    FieldName::Custom(name) => name,
                                    FieldName::Default => field_name_real.clone(),
                                };

                                pattern_token_stream
                                    .extend(quote!(#field_name_real: #field_name_var,));

                                let ty = &field.ty;

                                if let Some(method) = field_attribute.method {
                                    block_token_stream.extend(super::common::create_format_arg(
                                        ast,
                                        ty,
                                        &method,
                                        quote!(#field_name_var),
                                    ));

                                    block_token_stream.extend(if name_string.is_some() {
                                        quote! (builder.field(stringify!(#key), &arg);)
                                    } else {
                                        quote! (builder.entry(&Educe__RawString(stringify!(#key)), &arg);)
                                    });
                                } else {
                                    debug_types.push(ty);

                                    block_token_stream.extend(if name_string.is_some() {
                                        quote! (builder.field(stringify!(#key), #field_name_var);)
                                    } else {
                                        quote! (builder.entry(&Educe__RawString(stringify!(#key)), #field_name_var);)
                                    });
                                }

                                has_fields = true;
                            }
                        } else {
                            block_token_stream
                                .extend(quote!(let mut builder = f.debug_tuple(#name_string);));

                            for field in fields.named.iter() {
                                let field_attribute = FieldAttributeBuilder {
                                    enable_name:   false,
                                    enable_ignore: true,
                                    enable_method: true,
                                    name:          FieldName::Default,
                                }
                                .build_from_attributes(&field.attrs, traits)?;

                                let field_name_real = field.ident.as_ref().unwrap();
                                let field_name_var = format_ident!("_{}", field_name_real);

                                if field_attribute.ignore {
                                    pattern_token_stream.extend(quote!(#field_name_real: _,));

                                    continue;
                                }

                                pattern_token_stream
                                    .extend(quote!(#field_name_real: #field_name_var,));

                                let ty = &field.ty;

                                if let Some(method) = field_attribute.method {
                                    block_token_stream.extend(super::common::create_format_arg(
                                        ast,
                                        ty,
                                        &method,
                                        quote!(#field_name_var),
                                    ));

                                    block_token_stream.extend(quote! (builder.field(&arg);));
                                } else {
                                    debug_types.push(ty);

                                    block_token_stream
                                        .extend(quote! (builder.field(#field_name_var);));
                                }

                                has_fields = true;
                            }
                        }

                        if !has_fields && name_string.is_none() {
                            return Err(super::panic::unit_struct_need_name(variant_ident));
                        }

                        arms_token_stream.extend(quote! {
                            Self::#variant_ident { #pattern_token_stream } => {
                                #block_token_stream

                                builder.finish()
                            },
                        });
                    },
                    Fields::Unnamed(fields) => {
                        let mut has_fields = false;

                        let mut pattern_token_stream = proc_macro2::TokenStream::new();
                        let mut block_token_stream = proc_macro2::TokenStream::new();

                        if named_field {
                            block_token_stream
                                .extend(create_named_field_builder(name_string.as_deref()));

                            for (index, field) in fields.unnamed.iter().enumerate() {
                                let field_attribute = FieldAttributeBuilder {
                                    enable_name:   true,
                                    enable_ignore: true,
                                    enable_method: true,
                                    name:          FieldName::Default,
                                }
                                .build_from_attributes(&field.attrs, traits)?;

                                if field_attribute.ignore {
                                    pattern_token_stream.extend(quote!(_,));

                                    continue;
                                }

                                let field_name_var = format_ident!("_{}", index);

                                let key = match field_attribute.name {
                                    FieldName::Custom(name) => name,
                                    FieldName::Default => field_name_var.clone(),
                                };

                                pattern_token_stream.extend(quote!(#field_name_var,));

                                let ty = &field.ty;

                                if let Some(method) = field_attribute.method {
                                    block_token_stream.extend(super::common::create_format_arg(
                                        ast,
                                        ty,
                                        &method,
                                        quote!(#field_name_var),
                                    ));

                                    block_token_stream.extend(if name_string.is_some() {
                                        quote! (builder.field(stringify!(#key), &arg);)
                                    } else {
                                        quote! (builder.entry(&Educe__RawString(stringify!(#key)), &arg);)
                                    });
                                } else {
                                    debug_types.push(ty);

                                    block_token_stream.extend(if name_string.is_some() {
                                        quote! (builder.field(stringify!(#key), #field_name_var);)
                                    } else {
                                        quote! (builder.entry(&Educe__RawString(stringify!(#key)), #field_name_var);)
                                    });
                                }

                                has_fields = true;
                            }
                        } else {
                            block_token_stream
                                .extend(quote!(let mut builder = f.debug_tuple(#name_string);));

                            for (index, field) in fields.unnamed.iter().enumerate() {
                                let field_attribute = FieldAttributeBuilder {
                                    enable_name:   false,
                                    enable_ignore: true,
                                    enable_method: true,
                                    name:          FieldName::Default,
                                }
                                .build_from_attributes(&field.attrs, traits)?;

                                if field_attribute.ignore {
                                    pattern_token_stream.extend(quote!(_,));

                                    continue;
                                }

                                let field_name_var = format_ident!("_{}", index);

                                pattern_token_stream.extend(quote!(#field_name_var,));

                                let ty = &field.ty;

                                if let Some(method) = field_attribute.method {
                                    block_token_stream.extend(super::common::create_format_arg(
                                        ast,
                                        ty,
                                        &method,
                                        quote!(#field_name_var),
                                    ));

                                    block_token_stream.extend(quote! (builder.field(&arg);));
                                } else {
                                    debug_types.push(ty);

                                    block_token_stream
                                        .extend(quote! (builder.field(#field_name_var);));
                                }

                                has_fields = true;
                            }
                        }

                        if !has_fields && name_string.is_none() {
                            return Err(super::panic::unit_struct_need_name(variant_ident));
                        }

                        arms_token_stream.extend(quote! {
                            Self::#variant_ident ( #pattern_token_stream ) => {
                                #block_token_stream

                                builder.finish()
                            },
                        });
                    },
                }
            }
        }

        let ident = &ast.ident;

        if arms_token_stream.is_empty() {
            if let Some(ident) = name {
                builder_token_stream.extend(quote! {
                    f.write_str(stringify!(#ident))
                });
            } else {
                return Err(super::panic::unit_enum_need_name(ident));
            }
        } else {
            builder_token_stream.extend(quote! {
                match self {
                    #arms_token_stream
                }
            });
        }

        let bound = type_attribute.bound.into_where_predicates_by_generic_parameters_check_types(
            &ast.generics.params,
            &syn::parse2(quote!(::core::fmt::Debug)).unwrap(),
            &debug_types,
            &[],
        );

        let mut generics = ast.generics.clone();
        let where_clause = generics.make_where_clause();

        for where_predicate in bound {
            where_clause.predicates.push(where_predicate);
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        token_stream.extend(quote! {
            impl #impl_generics ::core::fmt::Debug for #ident #ty_generics #where_clause {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    #builder_token_stream
                }
            }
        });

        Ok(())
    }
}

#[inline]
fn create_named_field_builder(name_string: Option<&str>) -> proc_macro2::TokenStream {
    if let Some(name_string) = name_string {
        quote!(let mut builder = f.debug_struct(#name_string);)
    } else {
        super::common::create_debug_map_builder()
    }
}
