#[cfg(not(any(
    feature = "Debug",
    feature = "Clone",
    feature = "Copy",
    feature = "PartialEq",
    feature = "Eq",
    feature = "PartialOrd",
    feature = "Ord",
    feature = "Hash",
    feature = "Default",
    feature = "Deref",
    feature = "DerefMut",
    feature = "Into",
)))]
compile_error!("at least one of the trait features must be enabled");

use enum_ordinalize::Ordinalize;
use syn::Path;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ordinalize)]
#[ordinalize(impl_trait = false)]
#[ordinalize(variants(pub(crate) const VARIANTS))]
pub(crate) enum Trait {
    #[cfg(feature = "Debug")]
    Debug,
    #[cfg(feature = "Clone")]
    Clone,
    #[cfg(feature = "Copy")]
    Copy,
    #[cfg(feature = "PartialEq")]
    PartialEq,
    #[cfg(feature = "Eq")]
    Eq,
    #[cfg(feature = "PartialOrd")]
    PartialOrd,
    #[cfg(feature = "Ord")]
    Ord,
    #[cfg(feature = "Hash")]
    Hash,
    #[cfg(feature = "Default")]
    Default,
    #[cfg(feature = "Deref")]
    Deref,
    #[cfg(feature = "DerefMut")]
    DerefMut,
    #[cfg(feature = "Into")]
    Into,

    _Nothing,
}

impl Trait {
    #[inline]
    pub(crate) fn from_path(path: &Path) -> Option<Self> {
        let ident_string = match path.get_ident() {
            Some(ident) => ident.to_string(),
            None => return None,
        };

        match ident_string.as_str() {
            #[cfg(feature = "Debug")]
            "Debug" => Some(Self::Debug),
            #[cfg(feature = "Clone")]
            "Clone" => Some(Self::Clone),
            #[cfg(feature = "Copy")]
            "Copy" => Some(Self::Copy),
            #[cfg(feature = "PartialEq")]
            "PartialEq" => Some(Self::PartialEq),
            #[cfg(feature = "Eq")]
            "Eq" => Some(Self::Eq),
            #[cfg(feature = "PartialOrd")]
            "PartialOrd" => Some(Self::PartialOrd),
            #[cfg(feature = "Ord")]
            "Ord" => Some(Self::Ord),
            #[cfg(feature = "Hash")]
            "Hash" => Some(Self::Hash),
            #[cfg(feature = "Default")]
            "Default" => Some(Self::Default),
            #[cfg(feature = "Deref")]
            "Deref" => Some(Self::Deref),
            #[cfg(feature = "DerefMut")]
            "DerefMut" => Some(Self::DerefMut),
            #[cfg(feature = "Into")]
            "Into" => Some(Self::Into),
            _ => None,
        }
    }
}
