use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Meta, Token,
};

pub(crate) struct UnsafePunctuatedMeta {
    pub(crate) list:       Punctuated<Meta, Token![,]>,
    pub(crate) has_unsafe: bool,
}

impl Parse for UnsafePunctuatedMeta {
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let has_unsafe = input.parse::<Token![unsafe]>().is_ok();

        if input.is_empty() {
            return Ok(Self {
                list: Punctuated::new(),
                has_unsafe,
            });
        }

        if has_unsafe {
            input.parse::<Token![,]>()?;
        }

        let list = input.parse_terminated(Meta::parse, Token![,])?;

        Ok(Self {
            list,
            has_unsafe,
        })
    }
}
