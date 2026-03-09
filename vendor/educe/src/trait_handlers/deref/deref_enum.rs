use quote::{format_ident, quote};
use syn::{spanned::Spanned, Data, DeriveInput, Field, Fields, Ident, Meta, Type};

use super::{
    models::{FieldAttributeBuilder, TypeAttributeBuilder},
    TraitHandler,
};
use crate::{common::r#type::dereference, panic, supported_traits::Trait};

pub(crate) struct DerefEnumHandler;

impl TraitHandler for DerefEnumHandler {
    #[inline]
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        let _ = TypeAttributeBuilder {
            enable_flag: true
        }
        .build_from_deref_meta(meta)?;

        let mut target_token_stream = proc_macro2::TokenStream::new();
        let mut arms_token_stream = proc_macro2::TokenStream::new();

        if let Data::Enum(data) = &ast.data {
            type Variants<'a> = Vec<(&'a Ident, bool, usize, Ident, &'a Type)>;

            let mut variants: Variants = Vec::new();

            for variant in data.variants.iter() {
                let _ = TypeAttributeBuilder {
                    enable_flag: false
                }
                .build_from_attributes(&variant.attrs, traits)?;

                if let Fields::Unit = &variant.fields {
                    return Err(panic::trait_not_support_unit_variant(
                        meta.path().get_ident().unwrap(),
                        variant,
                    ));
                }

                let fields = &variant.fields;

                let (index, field) = if fields.len() == 1 {
                    let field = fields.into_iter().next().unwrap();

                    let _ = FieldAttributeBuilder {
                        enable_flag: true
                    }
                    .build_from_attributes(&field.attrs, traits)?;

                    (0usize, field)
                } else {
                    let mut deref_field: Option<(usize, &Field)> = None;

                    for (index, field) in variant.fields.iter().enumerate() {
                        let field_attribute = FieldAttributeBuilder {
                            enable_flag: true
                        }
                        .build_from_attributes(&field.attrs, traits)?;

                        if field_attribute.flag {
                            if deref_field.is_some() {
                                return Err(super::panic::multiple_deref_fields_of_variant(
                                    field_attribute.span,
                                    variant,
                                ));
                            }

                            deref_field = Some((index, field));
                        }
                    }

                    if let Some(deref_field) = deref_field {
                        deref_field
                    } else {
                        return Err(super::panic::no_deref_field_of_variant(meta.span(), variant));
                    }
                };

                let (field_name, is_tuple): (Ident, bool) = match field.ident.as_ref() {
                    Some(ident) => (ident.clone(), false),
                    None => (format_ident!("_{}", index), true),
                };

                variants.push((&variant.ident, is_tuple, index, field_name, &field.ty));
            }

            if variants.is_empty() {
                return Err(super::panic::no_deref_field(meta.span()));
            }

            let ty = variants[0].4;
            let dereference_ty = dereference(ty);

            target_token_stream.extend(quote!(#dereference_ty));

            for (variant_ident, is_tuple, index, field_name, _) in variants {
                let mut pattern_token_stream = proc_macro2::TokenStream::new();

                if is_tuple {
                    for _ in 0..index {
                        pattern_token_stream.extend(quote!(_,));
                    }

                    pattern_token_stream.extend(quote!( #field_name, .. ));

                    arms_token_stream.extend(
                        quote!( Self::#variant_ident ( #pattern_token_stream ) => #field_name, ),
                    );
                } else {
                    pattern_token_stream.extend(quote!( #field_name, .. ));

                    arms_token_stream.extend(
                        quote!( Self::#variant_ident { #pattern_token_stream } => #field_name, ),
                    );
                }
            }
        }

        let ident = &ast.ident;

        let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

        token_stream.extend(quote! {
            impl #impl_generics ::core::ops::Deref for #ident #ty_generics #where_clause {
                type Target = #target_token_stream;

                #[inline]
                fn deref(&self) -> &Self::Target {
                    match self {
                        #arms_token_stream
                    }
                }
            }
        });

        Ok(())
    }
}
