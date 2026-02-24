/// This trait provides an enum with the ability to not only obtain the ordinal values of its variants but also allows for the construction of enums from an ordinal value.
///
/// ```rust
/// use enum_ordinalize::Ordinalize;
///
/// #[repr(u8)]
/// enum E {
///     A,
///     B,
/// }
///
/// impl Ordinalize for E {
///     type VariantType = u8;
///
///     const VALUES: &'static [Self::VariantType] = &[0, 1];
///     const VARIANTS: &'static [Self] = &[E::A, E::B];
///     const VARIANT_COUNT: usize = 2;
///
///     #[inline]
///     unsafe fn from_ordinal_unsafe(number: Self::VariantType) -> Self {
///         ::core::mem::transmute(number)
///     }
///
///     #[inline]
///     fn from_ordinal(number: Self::VariantType) -> Option<Self> {
///         match number {
///             0 => Some(Self::A),
///             1 => Some(Self::B),
///             _ => None,
///         }
///     }
///
///     #[inline]
///     fn ordinal(&self) -> Self::VariantType {
///         match self {
///             Self::A => 0,
///             Self::B => 1,
///         }
///     }
/// }
/// ```
pub trait Ordinalize: Sized + 'static {
    /// The type of the values of the variants.
    type VariantType;

    /// The count of variants.
    const VARIANT_COUNT: usize;

    /// List of this enum's variants.
    const VARIANTS: &'static [Self];

    /// List of values for all variants of this enum.
    const VALUES: &'static [Self::VariantType];

    /// Obtain a variant based on an integer number.
    ///
    /// # Safety
    /// You have to ensure that the input integer number can correspond to a variant on your own.
    unsafe fn from_ordinal_unsafe(number: Self::VariantType) -> Self;

    /// Obtain a variant based on an integer number.
    fn from_ordinal(number: Self::VariantType) -> Option<Self>
    where
        Self: Sized;

    /// Retrieve the integer number of this variant.
    fn ordinal(&self) -> Self::VariantType;
}
