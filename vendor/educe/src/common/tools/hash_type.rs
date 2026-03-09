use std::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
    str::FromStr,
};

use proc_macro2::Span;
use quote::ToTokens;
use syn::{spanned::Spanned, Path, Type};

#[derive(Debug, Clone)]
pub(crate) struct HashType(String, Span);

impl PartialEq for HashType {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for HashType {}

impl PartialOrd for HashType {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HashType {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Hash for HashType {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.0, state);
    }
}

impl Display for HashType {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0.replace("& '", "&'"), f)
    }
}

impl From<Type> for HashType {
    #[inline]
    fn from(value: Type) -> Self {
        Self::from(&value)
    }
}

impl From<&Type> for HashType {
    #[inline]
    fn from(value: &Type) -> Self {
        Self(value.into_token_stream().to_string(), value.span())
    }
}

impl From<Path> for HashType {
    #[inline]
    fn from(value: Path) -> Self {
        Self::from(&value)
    }
}

impl From<&Path> for HashType {
    #[inline]
    fn from(value: &Path) -> Self {
        Self(value.into_token_stream().to_string(), value.span())
    }
}

#[allow(dead_code)]
impl HashType {
    #[inline]
    pub(crate) fn to_type(&self) -> Type {
        syn::parse_str(self.0.as_str()).unwrap()
    }

    #[inline]
    pub(crate) fn span(&self) -> Span {
        self.1
    }
}

impl ToTokens for HashType {
    #[inline]
    fn to_tokens(&self, token_stream: &mut proc_macro2::TokenStream) {
        let ty = proc_macro2::TokenStream::from_str(self.0.as_str()).unwrap();

        token_stream.extend(ty);
    }
}
