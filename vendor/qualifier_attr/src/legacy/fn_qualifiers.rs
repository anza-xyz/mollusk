use quote::ToTokens;
use syn::{parse::Parse, spanned::Spanned};
use proc_macro as pm;

enum FnQualifier {
    Visibility(syn::Visibility),
    Constness(syn::token::Const),
    Asyncness(syn::token::Async),
    Unsafety(syn::token::Unsafe),
    Abi(syn::Abi),
}

struct FnQualifiers {
    visibility: Option<syn::Visibility>,
    constness: Option<syn::token::Const>,
    asyncness: Option<syn::token::Async>,
    unsafety: Option<syn::token::Unsafe>,
    abi: Option<syn::Abi>,
}

enum FnQualifiersMeta {
    FnQualifiers(FnQualifiers),
}

impl Parse for FnQualifier {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::token::Pub) {
            Ok(FnQualifier::Visibility(input.parse()?))
        } else if input.peek(syn::token::Const) {
            Ok(FnQualifier::Constness(input.parse()?))
        } else if input.peek(syn::token::Async) {
            Ok(FnQualifier::Asyncness(input.parse()?))
        } else if input.peek(syn::token::Unsafe) {
            Ok(FnQualifier::Unsafety(input.parse()?))
        } else if input.peek(syn::token::Extern) {
            Ok(FnQualifier::Abi(input.parse()?))
        } else {
            Err(syn::Error::new(input.span(), "Expected a qualifier"))
        }
    }
}

impl Parse for FnQualifiers {
    // implement for parsing a list of qualifiers NOT enclosed in square brackets
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut visibility = None;
        let mut constness = None;
        let mut asyncness = None;
        let mut unsafety = None;
        let mut abi = None;

        while !input.is_empty() {
            let qualifier = input.parse::<FnQualifier>()?;
            match qualifier {
                FnQualifier::Visibility(vis) => {
                    if visibility.is_some() {
                        return Err(syn::Error::new(vis.span(), "Visibility already specified"));
                    }
                    visibility = Some(vis);
                }
                FnQualifier::Constness(constness_token) => {
                    if constness.is_some() {
                        return Err(syn::Error::new(constness_token.span(), "Constness already specified"));
                    }
                    constness = Some(constness_token);
                }
                FnQualifier::Asyncness(asyncness_token) => {
                    if asyncness.is_some() {
                        return Err(syn::Error::new(asyncness_token.span(), "Asyncness already specified"));
                    }
                    asyncness = Some(asyncness_token);
                }
                FnQualifier::Unsafety(unsafety_token) => {
                    if unsafety.is_some() {
                        return Err(syn::Error::new(unsafety_token.span(), "Unsafety already specified"));
                    }
                    unsafety = Some(unsafety_token);
                }
                FnQualifier::Abi(abi_token) => {
                    if abi.is_some() {
                        return Err(syn::Error::new(abi_token.span(), "Abi already specified"));
                    }
                    abi = Some(abi_token);
                }
            }
            if !input.is_empty() {
                input.parse::<syn::token::Comma>()?;
            }
        }

        Ok(FnQualifiers {
            visibility,
            constness,
            asyncness,
            unsafety,
            abi,
        })
    }
}

impl Parse for FnQualifiersMeta {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(FnQualifiersMeta::FnQualifiers(input.parse()?))
    }
}

pub(crate) fn fn_qualifiers(meta: pm::TokenStream, func: pm::TokenStream) -> pm::TokenStream {
    let meta = syn::parse_macro_input!(meta as FnQualifiersMeta);
    let mut func = syn::parse_macro_input!(func as syn::ItemFn);
    match meta {
        FnQualifiersMeta::FnQualifiers(fn_qualifiers) => {
            if let Some(visibility) = fn_qualifiers.visibility {
                func.vis = visibility;
            }
            func.sig.constness = fn_qualifiers.constness;
            func.sig.asyncness = fn_qualifiers.asyncness;
            func.sig.unsafety = fn_qualifiers.unsafety;
            func.sig.abi = fn_qualifiers.abi;
        }
    }

    func.into_token_stream().into()
}
