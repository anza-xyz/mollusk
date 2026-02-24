#[cfg(any(feature = "PartialOrd", feature = "Ord"))]
mod discriminant_type;

#[cfg(any(feature = "PartialOrd", feature = "Ord"))]
pub(crate) use discriminant_type::*;

#[cfg(feature = "Into")]
mod hash_type;

#[cfg(feature = "Into")]
pub(crate) use hash_type::*;
