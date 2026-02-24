use quote::quote;
use syn::{DeriveInput, Path, Type};

#[inline]
pub(crate) fn create_debug_map_builder() -> proc_macro2::TokenStream {
    quote!(
        #[allow(non_camel_case_types)] // We're using __ to help avoid clashes.
        struct Educe__RawString(&'static str);

        impl ::core::fmt::Debug for Educe__RawString {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.0)
            }
        }

        let mut builder = f.debug_map();
    )
}

#[inline]
pub(crate) fn create_format_arg(
    ast: &DeriveInput,
    field_ty: &Type,
    format_method: &Path,
    field_expr: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let ty_ident = &ast.ident;

    // We use the complete original generics, not filtered by field,
    // and include a PhantomData<Self> in our wrapper struct to use the generics.
    //
    // This avoids having to try to calculate the right *subset* of the generics
    // relevant for this field, which is nontrivial and maybe impossible.
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote!(
        let arg = {
            #[allow(non_camel_case_types)] // We're using __ to help avoid clashes.
            struct Educe__DebugField<V, M>(V, ::core::marker::PhantomData<M>);

            impl #impl_generics ::core::fmt::Debug
                for Educe__DebugField<&#field_ty, #ty_ident #ty_generics>
                #where_clause
            {
                #[inline]
                fn fmt(&self, educe__f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    #format_method(self.0, educe__f)
                }
            }

            Educe__DebugField(#field_expr, ::core::marker::PhantomData::<Self>)
        };
    )
}
