use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Meta, Type};

use super::{
    models::{FieldAttributeBuilder, FieldName, TypeAttributeBuilder, TypeName},
    TraitHandler,
};
use crate::{common::ident_index::IdentOrIndex, Trait};

pub struct DebugStructHandler;

impl TraitHandler for DebugStructHandler {
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        let is_tuple = {
            if let Data::Struct(data) = &ast.data {
                matches!(data.fields, Fields::Unnamed(_))
            } else {
                true
            }
        };

        let type_attribute = TypeAttributeBuilder {
            enable_flag:        true,
            enable_unsafe:      false,
            enable_name:        true,
            enable_named_field: true,
            enable_bound:       true,
            name:               TypeName::Default,
            named_field:        !is_tuple,
        }
        .build_from_debug_meta(meta)?;

        let name = type_attribute.name.to_ident_by_ident(&ast.ident);

        let mut debug_types: Vec<&Type> = Vec::new();

        let mut builder_token_stream = proc_macro2::TokenStream::new();
        let mut has_fields = false;

        if type_attribute.named_field {
            builder_token_stream.extend(if let Some(name) = name {
                quote!(let mut builder = f.debug_struct(stringify!(#name));)
            } else {
                super::common::create_debug_map_builder()
            });

            if let Data::Struct(data) = &ast.data {
                for (index, field) in data.fields.iter().enumerate() {
                    let field_attribute = FieldAttributeBuilder {
                        enable_name:   true,
                        enable_ignore: true,
                        enable_method: true,
                        name:          FieldName::Default,
                    }
                    .build_from_attributes(&field.attrs, traits)?;

                    if field_attribute.ignore {
                        continue;
                    }

                    let (key, field_name) = match field_attribute.name {
                        FieldName::Custom(name) => {
                            (name, IdentOrIndex::from_ident_with_index(field.ident.as_ref(), index))
                        },
                        FieldName::Default => {
                            if let Some(ident) = field.ident.as_ref() {
                                (ident.clone(), IdentOrIndex::from(ident))
                            } else {
                                (format_ident!("_{}", index), IdentOrIndex::from(index))
                            }
                        },
                    };

                    let ty = &field.ty;

                    if let Some(method) = field_attribute.method {
                        builder_token_stream.extend(super::common::create_format_arg(
                            ast,
                            ty,
                            &method,
                            quote!(&self.#field_name),
                        ));

                        builder_token_stream.extend(if name.is_some() {
                            quote! (builder.field(stringify!(#key), &arg);)
                        } else {
                            quote! (builder.entry(&Educe__RawString(stringify!(#key)), &arg);)
                        });
                    } else {
                        debug_types.push(ty);

                        builder_token_stream.extend(if name.is_some() {
                            quote! (builder.field(stringify!(#key), &self.#field_name);)
                        } else {
                            quote! (builder.entry(&Educe__RawString(stringify!(#key)), &self.#field_name);)
                        });
                    }

                    has_fields = true;
                }
            }
        } else {
            builder_token_stream
                .extend(quote!(let mut builder = f.debug_tuple(stringify!(#name));));

            if let Data::Struct(data) = &ast.data {
                for (index, field) in data.fields.iter().enumerate() {
                    let field_attribute = FieldAttributeBuilder {
                        enable_name:   false,
                        enable_ignore: true,
                        enable_method: true,
                        name:          FieldName::Default,
                    }
                    .build_from_attributes(&field.attrs, traits)?;

                    if field_attribute.ignore {
                        continue;
                    }

                    let field_name =
                        IdentOrIndex::from_ident_with_index(field.ident.as_ref(), index);

                    let ty = &field.ty;

                    if let Some(method) = field_attribute.method {
                        builder_token_stream.extend(super::common::create_format_arg(
                            ast,
                            ty,
                            &method,
                            quote!(&self.#field_name),
                        ));

                        builder_token_stream.extend(quote! (builder.field(&arg);));
                    } else {
                        debug_types.push(ty);

                        builder_token_stream.extend(quote! (builder.field(&self.#field_name);));
                    }

                    has_fields = true;
                }
            }
        }

        let ident = &ast.ident;

        if !has_fields && name.is_none() {
            return Err(super::panic::unit_struct_need_name(ident));
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

                    builder.finish()
                }
            }
        });

        Ok(())
    }
}
