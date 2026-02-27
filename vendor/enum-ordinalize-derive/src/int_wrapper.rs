use proc_macro2::{Literal, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::Expr;

use crate::int128::Int128;

pub(crate) enum IntWrapper {
    Integer(Int128),
    Constant(Expr, usize),
}

impl From<Int128> for IntWrapper {
    #[inline]
    fn from(v: Int128) -> IntWrapper {
        Self::Integer(v)
    }
}

impl From<i128> for IntWrapper {
    #[inline]
    fn from(v: i128) -> IntWrapper {
        Self::Integer(Int128::from(v))
    }
}

impl From<(&Expr, usize)> for IntWrapper {
    #[inline]
    fn from((expr, counter): (&Expr, usize)) -> IntWrapper {
        Self::Constant(expr.clone(), counter)
    }
}

impl ToTokens for IntWrapper {
    #[inline]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Integer(v) => {
                let lit = match v {
                    Int128::Signed(i) => Literal::i128_unsuffixed(*i),
                    Int128::Unsigned(u) => Literal::u128_unsuffixed(*u),
                };

                tokens.append(lit);
            },
            Self::Constant(expr, counter) => {
                let counter = *counter;

                if counter > 0 {
                    tokens.extend(quote!(#expr +));
                    tokens.append(Literal::usize_unsuffixed(counter));
                } else {
                    tokens.extend(quote!(#expr));
                }
            },
        }
    }
}
