extern crate proc_macro as pm;
extern crate proc_macro2 as pm2;

use std::collections::HashMap;

use quote::ToTokens;
use syn::{ext::IdentExt, spanned::Spanned, Field, Fields, Item, ItemStruct, ItemUnion};

use crate::{
    helper::Qualify,
    parse::{
        FieldQualifiers, FlexibleItemConst, FlexibleItemFn, FlexibleItemStatic, FlexibleItemType,
        Qualifiers,
    },
};

mod helper;
mod parse;
#[cfg(feature = "legacy_attrs")]
mod legacy;

#[proc_macro_attribute]
pub fn qualifiers(meta: pm::TokenStream, input: pm::TokenStream) -> pm::TokenStream {
    fn inner(meta: pm::TokenStream, input: pm::TokenStream) -> syn::Result<pm::TokenStream> {
        let qualifiers = syn::parse::<Qualifiers>(meta)?;

        // Try "flexible" items first.
        if let Ok(mut input) = syn::parse::<FlexibleItemConst>(input.clone()) {
            input.qualify().apply(qualifiers)?;
            return Ok(input.into_token_stream().into());
        }

        if let Ok(mut input) = syn::parse::<FlexibleItemFn>(input.clone()) {
            input.qualify().apply(qualifiers)?;
            return Ok(input.into_token_stream().into());
        }

        if let Ok(mut input) = syn::parse::<FlexibleItemStatic>(input.clone()) {
            input.qualify().apply(qualifiers)?;
            return Ok(input.into_token_stream().into());
        }

        if let Ok(mut input) = syn::parse::<FlexibleItemType>(input.clone()) {
            input.qualify().apply(qualifiers)?;
            return Ok(input.into_token_stream().into());
        }

        // Fallback to normal items.
        let mut input = syn::parse::<Item>(input)?;
        input.qualify().apply(qualifiers)?;
        Ok(input.into_token_stream().into())
    }

    match inner(meta, input) {
        Ok(output) => output,
        Err(error) => error.into_compile_error().into(),
    }
}

#[proc_macro_attribute]
#[cfg(feature = "legacy_attrs")]
pub fn fn_qualifiers(meta: pm::TokenStream, input: pm::TokenStream) -> pm::TokenStream {
    legacy::fn_qualifiers(meta, input)
}

#[proc_macro_attribute]
#[cfg(feature = "legacy_attrs")]
pub fn mod_qualifiers(meta: pm::TokenStream, input: pm::TokenStream) -> pm::TokenStream {
    legacy::mod_qualifiers(meta, input)
}

#[proc_macro_attribute]
#[cfg(feature = "legacy_attrs")]
pub fn struct_qualifiers(meta: pm::TokenStream, input: pm::TokenStream) -> pm::TokenStream {
    legacy::struct_qualifiers(meta, input)
}

#[proc_macro_attribute]
#[cfg(feature = "legacy_attrs")]
pub fn named_field_qualifiers(meta: pm::TokenStream, input: pm::TokenStream) -> pm::TokenStream {
    legacy::named_field_qualifiers(meta, input)
}

#[proc_macro_attribute]
pub fn field_qualifiers(meta: pm::TokenStream, input: pm::TokenStream) -> pm::TokenStream {
    fn inner(meta: pm::TokenStream, input: pm::TokenStream) -> syn::Result<pm::TokenStream> {
        let FieldQualifiers(field_qualifiers) = syn::parse::<FieldQualifiers>(meta)?;
        let mut input = syn::parse::<Item>(input)?;

        // NOTE: Remember to `.unraw()` field identifiers here! Otherwise, the
        // usage of raw identifiers may cause unexpected behavior. For example,
        // if `r#x` is used as the field name in the attribute, but `x` is the
        // actual field name, if `.unraw()` is not used, the field will not be
        // recognized!

        // NOTE: Due to this implementation, if a duplicate field is present,
        // only the last duplicate will be chosen. This isn't really a problem
        // though, since duplicate fields are an error anyways. Because of span
        // crimes however, the error message does look a little strange.

        // TODO: Maybe choose the first field instead of the last, even though
        // it doesn't really matter?

        let mut fields: HashMap<String, &mut Field> = match &mut input {
            Item::Struct(ItemStruct {
                fields: Fields::Named(fields),
                ..
            })
            | Item::Union(ItemUnion { fields, .. }) => fields
                .named
                .iter_mut()
                .map(|field| (field.ident.as_ref().unwrap().unraw().to_string(), field))
                .collect(),
            Item::Struct(ItemStruct {
                fields: Fields::Unnamed(fields),
                ..
            }) => fields
                .unnamed
                .iter_mut()
                .enumerate()
                .map(|(i, field)| (format!("_{}", i), field))
                .collect(),
            Item::Struct(ItemStruct {
                fields: Fields::Unit,
                ..
            }) => HashMap::new(),
            _ => {
                return Err(syn::Error::new(
                    input.span(),
                    "this item does not support field qualifiers",
                ));
            }
        };

        let mut errors = Vec::new();
        for (name, qualifiers) in field_qualifiers {
            if let Some(field) = fields.get_mut(&name.unraw().to_string()) {
                if let Err(error) = field.qualify().apply(qualifiers) {
                    errors.push(error);
                }
            } else {
                errors.push(syn::Error::new(
                    name.span(),
                    format!("unknown field `{}`", name),
                ));
            }
        }

        if let Some(error) = errors.into_iter().reduce(|mut error, next| {
            error.combine(next);
            error
        }) {
            Err(error)
        } else {
            Ok(input.into_token_stream().into())
        }
    }

    match inner(meta, input) {
        Ok(output) => output,
        Err(error) => error.into_compile_error().into(),
    }
}
