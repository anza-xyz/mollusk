use syn::{punctuated::Punctuated, token::Comma, GenericParam, Meta, Path, Type, WherePredicate};

use crate::common::where_predicates_bool::{
    create_where_predicates_from_all_generic_parameters,
    create_where_predicates_from_generic_parameters_check_types, meta_2_where_predicates,
    WherePredicates, WherePredicatesOrBool,
};

pub(crate) enum Bound {
    Disabled,
    Auto,
    Custom(WherePredicates),
    All,
}

impl Bound {
    #[inline]
    pub(crate) fn from_meta(meta: &Meta) -> syn::Result<Self> {
        debug_assert!(meta.path().is_ident("bound"));

        Ok(match meta_2_where_predicates(meta)? {
            WherePredicatesOrBool::WherePredicates(where_predicates) => {
                Self::Custom(where_predicates)
            },
            WherePredicatesOrBool::Bool(b) => {
                if b {
                    Self::Auto
                } else {
                    Self::Disabled
                }
            },
            WherePredicatesOrBool::All => Self::All,
        })
    }
}

impl Bound {
    #[inline]
    pub(crate) fn into_where_predicates_by_generic_parameters_check_types(
        self,
        params: &Punctuated<GenericParam, Comma>,
        bound_trait: &Path,
        types: &[&Type],
        supertraits: &[proc_macro2::TokenStream],
    ) -> Punctuated<WherePredicate, Comma> {
        match self {
            Self::Disabled => Punctuated::new(),
            Self::Auto => create_where_predicates_from_generic_parameters_check_types(
                bound_trait,
                types,
                supertraits,
            ),
            Self::Custom(where_predicates) => where_predicates,
            Self::All => create_where_predicates_from_all_generic_parameters(params, bound_trait),
        }
    }
}
