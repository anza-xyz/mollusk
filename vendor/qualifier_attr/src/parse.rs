use quote::{ToTokens, TokenStreamExt};

use syn::{
    braced,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use syn::{
    punctuated::Punctuated, token::Brace, Abi, AttrStyle, Attribute, Block, Expr, Generics, Ident,
    Signature, StaticMutability, Token, Type, TypeParamBound, Visibility,
};

use syn::{ForeignItemFn, ForeignItemStatic, ForeignItemType};
use syn::{ImplItemConst, ImplItemFn, ImplItemType};
use syn::{ItemConst, ItemFn, ItemStatic, ItemType};
use syn::{TraitItemConst, TraitItemFn, TraitItemType};

/// A qualifier.
#[derive(Clone)]
pub enum Qualifier {
    Visibility(Visibility),
    Defaultness(Token![default]),
    Constness(Token![const]),
    Asyncness(Token![async]),
    Unsafety(Token![unsafe]),
    Abi(Abi),
}

impl Parse for Qualifier {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![pub]) {
            input.parse().map(Self::Visibility)
        } else if input.peek(Token![default]) {
            input.parse().map(Self::Defaultness)
        } else if input.peek(Token![const]) {
            input.parse().map(Self::Constness)
        } else if input.peek(Token![async]) {
            input.parse().map(Self::Asyncness)
        } else if input.peek(Token![unsafe]) {
            input.parse().map(Self::Unsafety)
        } else if input.peek(Token![extern]) {
            input.parse().map(Self::Abi)
        } else {
            Err(syn::Error::new(input.span(), "expected a qualifier"))
        }
    }
}

/// A set of qualifiers.
#[derive(Clone)]
pub struct Qualifiers {
    pub visibility: Option<Visibility>,
    pub defaultness: Option<Token![default]>,
    pub constness: Option<Token![const]>,
    pub asyncness: Option<Token![async]>,
    pub unsafety: Option<Token![unsafe]>,
    pub abi: Option<Abi>,
}

impl Parse for Qualifiers {
    // implement for parsing a list of qualifiers NOT enclosed in square brackets
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut visibility = None;
        let mut defaultness = None;
        let mut constness = None;
        let mut asyncness = None;
        let mut unsafety = None;
        let mut abi = None;

        while !input.is_empty() {
            let qualifier = input.parse::<Qualifier>()?;
            match qualifier {
                Qualifier::Visibility(visibility_token) => {
                    if visibility.is_some() {
                        return Err(syn::Error::new(
                            visibility_token.span(),
                            "visibility already specified",
                        ));
                    }
                    visibility = Some(visibility_token);
                }
                Qualifier::Defaultness(defaultness_token) => {
                    if defaultness.is_some() {
                        return Err(syn::Error::new(
                            defaultness_token.span(),
                            "defaultness already specified",
                        ));
                    }
                    defaultness = Some(defaultness_token);
                }
                Qualifier::Constness(constness_token) => {
                    if constness.is_some() {
                        return Err(syn::Error::new(
                            constness_token.span(),
                            "constness already specified",
                        ));
                    }
                    constness = Some(constness_token);
                }
                Qualifier::Asyncness(asyncness_token) => {
                    if asyncness.is_some() {
                        return Err(syn::Error::new(
                            asyncness_token.span(),
                            "asyncness already specified",
                        ));
                    }
                    asyncness = Some(asyncness_token);
                }
                Qualifier::Unsafety(unsafety_token) => {
                    if unsafety.is_some() {
                        return Err(syn::Error::new(
                            unsafety_token.span(),
                            "unsafety already specified",
                        ));
                    }
                    unsafety = Some(unsafety_token);
                }
                Qualifier::Abi(abi_token) => {
                    if abi.is_some() {
                        return Err(syn::Error::new(abi_token.span(), "ABI already specified"));
                    }
                    abi = Some(abi_token);
                }
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            visibility,
            defaultness,
            constness,
            asyncness,
            unsafety,
            abi,
        })
    }
}

/// A list of field names and the qualifiers to apply to them.
#[derive(Clone)]
pub struct FieldQualifiers(pub Vec<(Ident, Qualifiers)>);

impl Parse for FieldQualifiers {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut list = Vec::new();

        while !input.is_empty() {
            let name = input.parse::<Ident>()?;
            let content;
            let _paren_token = parenthesized!(content in input);
            let qualifiers = content.parse::<Qualifiers>()?;
            list.push((name, qualifiers));
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self(list))
    }
}

/// A combination of [`ItemConst`], [`ImplItemConst`], and [`TraitItemConst`].
#[derive(Clone)]
pub struct FlexibleItemConst {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub defaultness: Option<Token![default]>,
    pub const_token: Token![const],
    pub ident: Ident,
    pub generics: Generics,
    pub colon_token: Token![:],
    pub ty: Type,
    pub body: Option<(Token![=], Expr)>,
    pub semi_token: Token![;],
}

impl Parse for FlexibleItemConst {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            vis: input.parse()?,
            defaultness: input.parse()?,
            const_token: input.parse()?,
            ident: {
                let lookahead = input.lookahead1();
                if lookahead.peek(Ident) || lookahead.peek(Token![_]) {
                    input.call(Ident::parse_any)?
                } else {
                    return Err(lookahead.error());
                }
            },
            generics: Generics::default(),
            colon_token: input.parse()?,
            ty: input.parse()?,
            body: if input.peek(Token![=]) {
                let eq_token = input.parse::<Token![=]>()?;
                let expr = input.parse::<Expr>()?;
                Some((eq_token, expr))
            } else {
                None
            },
            semi_token: input.parse()?,
        })
    }
}

impl ToTokens for FlexibleItemConst {
    fn to_tokens(&self, tokens: &mut pm2::TokenStream) {
        tokens.append_all(&self.attrs);
        self.vis.to_tokens(tokens);
        self.defaultness.to_tokens(tokens);
        self.const_token.to_tokens(tokens);
        self.ident.to_tokens(tokens);
        self.generics.to_tokens(tokens);
        self.colon_token.to_tokens(tokens);
        self.ty.to_tokens(tokens);
        if let Some((eq_token, expr)) = &self.body {
            eq_token.to_tokens(tokens);
            expr.to_tokens(tokens);
        }
        self.semi_token.to_tokens(tokens);
    }
}

impl From<ItemConst> for FlexibleItemConst {
    fn from(item_const: ItemConst) -> Self {
        Self {
            attrs: item_const.attrs,
            vis: item_const.vis,
            defaultness: None,
            const_token: item_const.const_token,
            ident: item_const.ident,
            generics: item_const.generics,
            colon_token: item_const.colon_token,
            ty: *item_const.ty,
            body: Some((item_const.eq_token, *item_const.expr)),
            semi_token: item_const.semi_token,
        }
    }
}

impl From<ImplItemConst> for FlexibleItemConst {
    fn from(item_const: ImplItemConst) -> Self {
        Self {
            attrs: item_const.attrs,
            vis: item_const.vis,
            defaultness: item_const.defaultness,
            const_token: item_const.const_token,
            ident: item_const.ident,
            generics: item_const.generics,
            colon_token: item_const.colon_token,
            ty: item_const.ty,
            body: Some((item_const.eq_token, item_const.expr)),
            semi_token: item_const.semi_token,
        }
    }
}

impl From<TraitItemConst> for FlexibleItemConst {
    fn from(item_const: TraitItemConst) -> Self {
        Self {
            attrs: item_const.attrs,
            vis: Visibility::Inherited,
            defaultness: None,
            const_token: item_const.const_token,
            ident: item_const.ident,
            generics: item_const.generics,
            colon_token: item_const.colon_token,
            ty: item_const.ty,
            body: item_const.default,
            semi_token: item_const.semi_token,
        }
    }
}

/// A combination of [`ItemFn`], [`ForeignItemFn`], [`ImplItemFn`], and [`TraitItemFn`].
#[derive(Clone)]
pub struct FlexibleItemFn {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub defaultness: Option<Token![default]>,
    pub sig: Signature,
    pub body: Option<Block>,
    pub semi_token: Option<Token![;]>,
}

impl Parse for FlexibleItemFn {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let defaultness = input.parse()?;
        let sig = input.parse()?;

        let lookahead = input.lookahead1();
        let (brace_token, stmts, semi_token) = if lookahead.peek(Brace) {
            let content;
            let brace_token = braced!(content in input);
            attrs.extend(content.call(Attribute::parse_inner)?);
            let stmts = content.call(Block::parse_within)?;
            (Some(brace_token), stmts, None)
        } else if lookahead.peek(Token![;]) {
            let semi_token = input.parse::<Token![;]>()?;
            (None, Vec::new(), Some(semi_token))
        } else {
            return Err(lookahead.error());
        };

        Ok(Self {
            attrs,
            vis,
            defaultness,
            sig,
            body: brace_token.map(|brace_token| Block { brace_token, stmts }),
            semi_token,
        })
    }
}

impl ToTokens for FlexibleItemFn {
    fn to_tokens(&self, tokens: &mut pm2::TokenStream) {
        tokens.append_all(
            self.attrs
                .iter()
                .filter(|attr| matches!(attr.style, AttrStyle::Outer)),
        );
        self.vis.to_tokens(tokens);
        self.defaultness.to_tokens(tokens);
        self.sig.to_tokens(tokens);
        if let Some(body) = &self.body {
            body.brace_token.surround(tokens, |tokens| {
                tokens.append_all(
                    self.attrs
                        .iter()
                        .filter(|attr| matches!(attr.style, AttrStyle::Inner(_))),
                );
                tokens.append_all(&body.stmts);
            });
        }
        self.semi_token.to_tokens(tokens);
    }
}

impl From<ItemFn> for FlexibleItemFn {
    fn from(item_fn: ItemFn) -> Self {
        Self {
            attrs: item_fn.attrs,
            vis: item_fn.vis,
            defaultness: None,
            sig: item_fn.sig,
            body: Some(*item_fn.block),
            semi_token: None,
        }
    }
}

impl From<ForeignItemFn> for FlexibleItemFn {
    fn from(item_fn: ForeignItemFn) -> Self {
        Self {
            attrs: item_fn.attrs,
            vis: item_fn.vis,
            defaultness: None,
            sig: item_fn.sig,
            body: None,
            semi_token: Some(item_fn.semi_token),
        }
    }
}

impl From<ImplItemFn> for FlexibleItemFn {
    fn from(item_fn: ImplItemFn) -> Self {
        Self {
            attrs: item_fn.attrs,
            vis: item_fn.vis,
            defaultness: item_fn.defaultness,
            sig: item_fn.sig,
            body: Some(item_fn.block),
            semi_token: None,
        }
    }
}

impl From<TraitItemFn> for FlexibleItemFn {
    fn from(item_fn: TraitItemFn) -> Self {
        Self {
            attrs: item_fn.attrs,
            vis: Visibility::Inherited,
            defaultness: None,
            sig: item_fn.sig,
            body: item_fn.default,
            semi_token: item_fn.semi_token,
        }
    }
}

/// A combination of [`ItemStatic`] and [`ForeignItemStatic`].
#[derive(Clone)]
pub struct FlexibleItemStatic {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub static_token: Token![static],
    pub mutability: StaticMutability,
    pub ident: Ident,
    pub colon_token: Token![:],
    pub ty: Type,
    pub body: Option<(Token![=], Expr)>,
    pub semi_token: Token![;],
}

impl Parse for FlexibleItemStatic {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            vis: input.parse()?,
            static_token: input.parse()?,
            mutability: input.parse()?,
            ident: input.parse()?,
            colon_token: input.parse()?,
            ty: input.parse()?,
            body: if input.peek(Token![=]) {
                let eq_token = input.parse::<Token![=]>()?;
                let expr = input.parse::<Expr>()?;
                Some((eq_token, expr))
            } else {
                None
            },
            semi_token: input.parse()?,
        })
    }
}

impl ToTokens for FlexibleItemStatic {
    fn to_tokens(&self, tokens: &mut pm2::TokenStream) {
        tokens.append_all(&self.attrs);
        self.vis.to_tokens(tokens);
        self.static_token.to_tokens(tokens);
        self.mutability.to_tokens(tokens);
        self.ident.to_tokens(tokens);
        self.colon_token.to_tokens(tokens);
        self.ty.to_tokens(tokens);
        if let Some((eq_token, expr)) = &self.body {
            eq_token.to_tokens(tokens);
            expr.to_tokens(tokens);
        }
        self.semi_token.to_tokens(tokens);
    }
}

impl From<ItemStatic> for FlexibleItemStatic {
    fn from(item_static: ItemStatic) -> Self {
        Self {
            attrs: item_static.attrs,
            vis: item_static.vis,
            static_token: item_static.static_token,
            mutability: item_static.mutability,
            ident: item_static.ident,
            colon_token: item_static.colon_token,
            ty: *item_static.ty,
            body: Some((item_static.eq_token, *item_static.expr)),
            semi_token: item_static.semi_token,
        }
    }
}

impl From<ForeignItemStatic> for FlexibleItemStatic {
    fn from(item_static: ForeignItemStatic) -> Self {
        Self {
            attrs: item_static.attrs,
            vis: item_static.vis,
            static_token: item_static.static_token,
            mutability: item_static.mutability,
            ident: item_static.ident,
            colon_token: item_static.colon_token,
            ty: *item_static.ty,
            body: None,
            semi_token: item_static.semi_token,
        }
    }
}

/// A combination of [`ItemType`], [`ForeignItemType`], [`ImplItemType`], and [`TraitItemType`].
#[derive(Clone)]
pub struct FlexibleItemType {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub defaultness: Option<Token![default]>,
    pub type_token: Token![type],
    pub ident: Ident,
    pub generics: Generics,
    pub colon_token: Option<Token![:]>,
    pub bounds: Punctuated<TypeParamBound, Token![+]>,
    pub body: Option<(Token![=], Type)>,
    pub semi_token: Token![;],
}

impl Parse for FlexibleItemType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let defaultness = input.parse()?;
        let type_token = input.parse()?;
        let ident = input.parse()?;
        let mut generics = input.parse::<Generics>()?;

        let colon_token = input.parse::<Option<Token![:]>>()?;
        let mut bounds = Punctuated::new();
        if colon_token.is_some() {
            loop {
                if input.peek(Token![where]) || input.peek(Token![=]) || input.peek(Token![;]) {
                    break;
                }
                bounds.push_value(input.parse::<TypeParamBound>()?);
                if input.peek(Token![where]) || input.peek(Token![=]) || input.peek(Token![;]) {
                    break;
                }
                bounds.push_punct(input.parse::<Token![+]>()?);
            }
        }

        let body = if input.peek(Token![=]) {
            let eq_token = input.parse::<Token![=]>()?;
            let ty = input.parse::<Type>()?;
            Some((eq_token, ty))
        } else {
            None
        };

        generics.where_clause = input.parse()?;
        let semi_token: Token![;] = input.parse()?;

        Ok(Self {
            attrs,
            vis,
            defaultness,
            type_token,
            ident,
            generics,
            colon_token,
            bounds,
            body,
            semi_token,
        })
    }
}

impl ToTokens for FlexibleItemType {
    fn to_tokens(&self, tokens: &mut pm2::TokenStream) {
        tokens.append_all(&self.attrs);
        self.vis.to_tokens(tokens);
        self.defaultness.to_tokens(tokens);
        self.type_token.to_tokens(tokens);
        self.ident.to_tokens(tokens);
        self.generics.to_tokens(tokens);
        self.colon_token.to_tokens(tokens);
        self.bounds.to_tokens(tokens);
        if let Some((eq_token, ty)) = &self.body {
            eq_token.to_tokens(tokens);
            ty.to_tokens(tokens);
        }
        self.generics.where_clause.to_tokens(tokens);
        self.semi_token.to_tokens(tokens);
    }
}

impl From<ItemType> for FlexibleItemType {
    fn from(item_type: ItemType) -> Self {
        Self {
            attrs: item_type.attrs,
            vis: item_type.vis,
            defaultness: None,
            type_token: item_type.type_token,
            ident: item_type.ident,
            generics: item_type.generics,
            colon_token: None,
            bounds: Punctuated::new(),
            body: Some((item_type.eq_token, *item_type.ty)),
            semi_token: item_type.semi_token,
        }
    }
}

impl From<ForeignItemType> for FlexibleItemType {
    fn from(item_type: ForeignItemType) -> Self {
        Self {
            attrs: item_type.attrs,
            vis: item_type.vis,
            defaultness: None,
            type_token: item_type.type_token,
            ident: item_type.ident,
            generics: item_type.generics,
            colon_token: None,
            bounds: Punctuated::new(),
            body: None,
            semi_token: item_type.semi_token,
        }
    }
}

impl From<ImplItemType> for FlexibleItemType {
    fn from(item_type: ImplItemType) -> Self {
        Self {
            attrs: item_type.attrs,
            vis: item_type.vis,
            defaultness: item_type.defaultness,
            type_token: item_type.type_token,
            ident: item_type.ident,
            generics: item_type.generics,
            colon_token: None,
            bounds: Punctuated::new(),
            body: Some((item_type.eq_token, item_type.ty)),
            semi_token: item_type.semi_token,
        }
    }
}

impl From<TraitItemType> for FlexibleItemType {
    fn from(item_type: TraitItemType) -> Self {
        Self {
            attrs: item_type.attrs,
            vis: Visibility::Inherited,
            defaultness: None,
            type_token: item_type.type_token,
            ident: item_type.ident,
            generics: item_type.generics,
            colon_token: item_type.colon_token,
            bounds: item_type.bounds,
            body: item_type.default,
            semi_token: item_type.semi_token,
        }
    }
}
