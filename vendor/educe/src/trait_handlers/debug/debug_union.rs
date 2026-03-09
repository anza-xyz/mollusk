use quote::quote;
use syn::{Data, DeriveInput, Meta};

use super::{
    models::{FieldAttributeBuilder, FieldName, TypeAttributeBuilder, TypeName},
    TraitHandler,
};
use crate::supported_traits::Trait;

pub(crate) struct DebugUnionHandler;

impl TraitHandler for DebugUnionHandler {
    fn trait_meta_handler(
        ast: &DeriveInput,
        token_stream: &mut proc_macro2::TokenStream,
        traits: &[Trait],
        meta: &Meta,
    ) -> syn::Result<()> {
        let type_attribute = TypeAttributeBuilder {
            enable_flag:        true,
            enable_unsafe:      true,
            enable_name:        true,
            enable_named_field: false,
            enable_bound:       false,
            name:               TypeName::Default,
            named_field:        false,
        }
        .build_from_debug_meta(meta)?;

        if !type_attribute.has_unsafe {
            return Err(super::panic::union_without_unsafe(meta));
        }

        let name = type_attribute.name.to_ident_by_ident(&ast.ident);

        let mut builder_token_stream = proc_macro2::TokenStream::new();

        if let Data::Union(data) = &ast.data {
            for field in data.fields.named.iter() {
                let _ = FieldAttributeBuilder {
                    enable_name:   false,
                    enable_ignore: false,
                    enable_method: false,
                    name:          FieldName::Default,
                }
                .build_from_attributes(&field.attrs, traits)?;
            }

            if let Some(name) = name {
                builder_token_stream.extend(quote!(
                    let mut builder = f.debug_tuple(stringify!(#name));

                    let size = ::core::mem::size_of::<Self>();

                    let data = unsafe { ::core::slice::from_raw_parts(self as *const Self as *const u8, size) };

                    builder.field(&data);

                    builder.finish()
                ));
            } else {
                builder_token_stream.extend(quote!(
                    let size = ::core::mem::size_of::<Self>();
                    let data = unsafe { ::core::slice::from_raw_parts(self as *const Self as *const u8, size) };

                    ::core::fmt::Debug::fmt(data, f)
                ));
            }
        }

        let ident = &ast.ident;

        let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

        token_stream.extend(quote! {
            impl #impl_generics ::core::fmt::Debug for #ident #ty_generics #where_clause {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    #builder_token_stream
                }
            }
        });

        Ok(())
    }
}
