use std::mem;
use std::ops;

/// Interface to primitive bit storage.
///
/// Types implementing this trait can be used as the blocks of a bit-vector.
pub trait BlockType: Copy +
                     PartialEq +
                     Ord +
                     ops::BitAnd<Output = Self> +
                     ops::BitOr<Output = Self> +
                     ops::BitXor<Output = Self> +
                     ops::Not<Output = Self> +
                     ops::Shl<usize, Output = Self> +
                     ops::Shr<usize, Output = Self> +
                     ops::Sub<Output = Self>
{
    /// The number of bits in a block.
    #[inline]
    fn nbits() -> usize {
        8 * mem::size_of::<Self>()
    }

    /// Returns `index / Self::nbits()`, computed by shifting.
    ///
    /// This is intended for converting a bit address into a block
    /// address, which is why it takes `u64` and returns `usize`.
    /// There is no check that the result actually fits in a `usize`,
    /// so this should only be used when `index` is already known to
    /// be small enough.
    #[inline]
    fn div_nbits(index: u64) -> usize {
        (index >> Self::lg_nbits()) as usize
    }

    /// Returns `index / Self::nbits()`, computed by shifting.
    ///
    /// This is intended for converting a bit address into a block
    /// address, which is why it takes `u64` and returns `usize`. It can only fail (returning
    /// `None`) if `usize` is 32 bits.
    #[inline]
    fn checked_div_nbits(index: u64) -> Option<usize> {
        (index >> Self::lg_nbits()).to_usize()
    }

    /// Returns `index / Self::nbits()` rounded up, computed by shifting.
    ///
    /// This is intended for converting a bit size into a block
    /// size, which is why it takes `u64` and returns `usize`.
    #[inline]
    fn ceil_div_nbits(index: u64) -> usize {
        Self::div_nbits(index + (Self::nbits() as u64 - 1))
    }

    /// Returns `index / Self::nbits()` rounded up, computed by shifting.
    ///
    /// This is intended for converting a bit size into a block
    /// size, which is why it takes `u64` and returns `usize`.
    /// There is no check that the result actually fits in a `usize`,
    /// so this should only be used when `index` is already known to
    /// be small enough.
    #[inline]
    fn checked_ceil_div_nbits(index: u64) -> Option<usize> {
        Self::checked_div_nbits(index + (Self::nbits() as u64 - 1))
    }

    /// Returns `index % Self::nbits()`, computed by masking.
    ///
    /// This is intended for converting a bit address into a bit offset
    /// within a block, which is why it takes `u64` and returns `usize`.
    #[inline]
    fn mod_nbits(index: u64) -> usize {
        let mask: u64 = Self::lg_nbits_mask();
        (index & mask) as usize
    }

    /// Returns `index * Self::nbits()`, computed by shifting.
    ///
    /// This is intended for converting a block address into a bit address,
    /// which is why it takes a `usize` and returns a `u64`.
    #[inline]
    fn mul_nbits(index: usize) -> u64 {
        (index as u64) << Self::lg_nbits()
    }

    /// The number of bits in the block at `position`, given a total bit length
    /// of `len`.
    ///
    /// This will be `Self::nbits()` for all but the last block, for which it may
    /// be less.
    ///
    /// # Precondition
    ///
    /// `position * Self::nbits() <= len`, or the block doesn't exist and the result
    /// is undefined.
    #[inline]
    fn block_bits(len: u64, position: usize) -> usize {
        let block_start = Self::mul_nbits(position);
        let block_limit = block_start + Self::nbits() as u64;

        debug_assert!( block_start <= len,
                       "BlockType::block_bits: precondition" );

        usize::if_then_else(
            block_limit <= len,
            Self::nbits(),
            len.wrapping_sub(block_start) as usize
        )
    }

    /// Log-base-2 of the number of bits in a block.
    #[inline]
    fn lg_nbits() -> usize {
        Self::nbits().floor_lg()
    }

    /// Mask with the lowest-order `lg_nbits()` set.
    #[inline]
    fn lg_nbits_mask<Result: BlockType>() -> Result {
        Result::low_mask(Self::lg_nbits())
    }

    /// The bit mask consisting of `Self::nbits() - element_bits` zeroes
    /// followed by `element_bits` ones.
    ///
    /// The default implementation has a branch, but should be overrided with
    /// a branchless algorithm if possible.
    ///
    /// # Precondition
    ///
    /// `element_bits <= Self::nbits()`
    #[inline]
    fn low_mask(element_bits: usize) -> Self {
        debug_assert!(element_bits <= Self::nbits());

        if element_bits == Self::nbits() {
            !Self::zero()
        } else {
            (Self::one() << element_bits) - Self::one()
        }
    }

    /// The bit mask with the `bit_index`th bit set.
    ///
    /// Bits are indexed in little-endian style based at 0.
    ///
    /// # Precondition
    ///
    /// `bit_index < Self::nbits()`
    #[inline]
    fn nth_mask(bit_index: usize) -> Self {
        Self::one() << bit_index
    }

    // Methods for getting and setting bits.

    /// Extracts the value of the `bit_index`th bit.
    ///
    /// # Panics
    ///
    /// Panics if `bit_index` is out of bounds.
    #[inline]
    fn get_bit(self, bit_index: usize) -> bool {
        assert!(bit_index < Self::nbits(), "Block::get_bit: out of bounds");
        self & Self::nth_mask(bit_index) != Self::zero()
    }

    /// Functionally updates the value of the `bit_index`th bit to `bit_value`.
    ///
    /// # Panics
    ///
    /// Panics if `bit_index` is out of bounds.
    #[inline]
    fn with_bit(self, bit_index: usize, bit_value: bool) -> Self {
        assert!(bit_index < Self::nbits(), "Block::with_bit: out of bounds");
        if bit_value {
            self | Self::nth_mask(bit_index)
        } else {
            self & !Self::nth_mask(bit_index)
        }
    }

    /// Extracts `len` bits starting at bit offset `start`.
    ///
    /// # Panics
    ///
    /// Panics of the bit span is out of bounds.
    #[inline]
    fn get_bits(self, start: usize, len: usize) -> Self {
        assert!(start + len <= Self::nbits(),
                "Block::get_bits: out of bounds");

        (self >> start) & Self::low_mask(len)
    }

    /// Functionally updates `len` bits to `value` starting at offset `start`.
    ///
    /// # Panics
    ///
    /// Panics of the bit span is out of bounds.
    #[inline]
    fn with_bits(self, start: usize, len: usize, value: Self) -> Self {
        assert!(start + len <= Self::nbits(),
                "Block::with_bits: out of bounds");

        let mask = Self::low_mask(len) << start;
        let shifted_value = value << start;

        (self & !mask) | (shifted_value & mask)
    }

    /// Returns the smallest number `n` such that `2.pow(n) >= self`.
    #[inline]
    fn ceil_lg(self) -> usize {
        usize::if_then(
            self > Self::one(),
            Self::nbits().wrapping_sub((self.wrapping_sub(Self::one())).leading_zeros() as usize)
        )
    }

    /// Returns the largest number `n` such that `2.pow(n) <= self`.
    #[inline]
    fn floor_lg(self) -> usize {
        usize::if_then(
            self > Self::one(),
            Self::nbits().wrapping_sub(1).wrapping_sub(self.leading_zeros() as usize)
        )
    }

    /// A shift-left operation that does not overflow.
    fn wrapping_shl(self, shift: u32) -> Self;

    /// A subtraction operation that does not overflow.
    fn wrapping_sub(self, other: Self) -> Self;

    /// Returns the number of leading zero bits in the given number.
    fn leading_zeros(self) -> usize;

    /// Converts the number to a `usize`, if it fits.
    fn to_usize(self) -> Option<usize>;

    /// Returns 0.
    fn zero() -> Self;

    /// Returns 1.
    fn one() -> Self;
}

trait IfThenElse {
    fn if_then_else(cond: bool, then_val: Self, else_val: Self) -> Self;
    fn if_then(cond: bool, then_val: Self) -> Self;
}

macro_rules! impl_block_type {
    ( $ty:ident )
        =>
    {
        impl IfThenElse for $ty {
            #[inline]
            fn if_then_else(cond: bool, then_val: Self, else_val: Self) -> Self {
                let then_cond = cond as Self;
                let else_cond = 1 - then_cond;
                (then_cond * then_val) | (else_cond * else_val)
            }

            #[inline]
            fn if_then(cond: bool, then_val: Self) -> Self {
                (cond as Self) * then_val
            }
        }

        impl BlockType for $ty {
            // The default `low_mask` has a branch, but we can do better if we have
            // `wrapping_shl`. That isn't a member of any trait, but all the primitive
            // numeric types have it, so we can override low_mask in this macro.
            #[inline]
            fn low_mask(k: usize) -> Self {
                debug_assert!(k <= Self::nbits());

                // Compute the mask when element_bits is not the word size:
                let a = Self::one().wrapping_shl(k as u32).wrapping_sub(1);

                // Special case for the word size:
                let b = (Self::div_nbits(k as u64) & 1) as Self * !0;

                a | b
            }

            #[inline]
            fn wrapping_shl(self, shift: u32) -> Self {
                self.wrapping_shl(shift)
            }

            #[inline]
            fn wrapping_sub(self, other: Self) -> Self {
                self.wrapping_sub(other)
            }

            #[inline]
            fn leading_zeros(self) -> usize {
                self.leading_zeros() as usize
            }

            #[inline]
            fn to_usize(self) -> Option<usize> {
                if self as usize as Self == self {
                    Some(self as usize)
                } else {
                    None
                }
            }

            #[inline]
            fn zero() -> Self {
                0
            }

            #[inline]
            fn one() -> Self {
                1
            }
        }
    }
}

impl_block_type!(u8);
impl_block_type!(u16);
impl_block_type!(u32);
impl_block_type!(u64);
#[cfg(int_128)]
impl_block_type!(u128);
impl_block_type!(usize);

/// Represents the address of a bit, broken into a block component
/// and a bit offset component.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Address {
    /// The index of the block containing the bit in question.
    pub block_index: usize,
    /// The position of the bit in question within its block.
    pub bit_offset: usize,
}

impl Address {
    /// Creates an `Address` for the given bit index for storage in
    /// block type `Block`.
    ///
    /// # Panics
    ///
    /// Panics if `bit_index` divided by the block size doesn’t fit in a
    /// `usize`.
    #[inline]
    pub fn new<Block: BlockType>(bit_index: u64) -> Self {
        Address {
            block_index: Block::checked_div_nbits(bit_index)
                .expect("Address::new: index overflow"),
            bit_offset: Block::mod_nbits(bit_index),
        }
    }

//    /// Converts an `Address` back into a raw bit index.
//    ///
//    /// This method and `new` should be inverses.
//    #[inline]
//    pub fn bit_index<Block: BlockType>(&self) -> u64 {
//        Block::mul_nbits(self.block_index) + self.bit_offset as u64
//    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{quickcheck, TestResult};

    #[test]
    fn nbits() {
        assert_eq!(8, u8::nbits());
        assert_eq!(16, u16::nbits());
        assert_eq!(32, u32::nbits());
        assert_eq!(64, u64::nbits());
    }

    quickcheck!{
        fn prop_div_nbits(n: u32) -> bool {
            u32::div_nbits(n as u64) == (n / 32) as usize
        }

        fn prop_ceil_div_nbits1(n: u32) -> bool {
            u32::ceil_div_nbits(n as u64) ==
                (n as f32 / 32.0f32).ceil() as usize
        }

        fn prop_ceil_div_nbits2(n: u32) -> bool {
            let result = u32::ceil_div_nbits(n as u64);
            result * 32 >= n as usize &&
                (result == 0 || (result - 1) * 32 < n as usize)
        }

        fn prop_mod_nbits(n: u32) -> bool {
            u32::mod_nbits(n as u64) == n as usize % 32
        }

        fn prop_mul_nbits(n: u32) -> bool {
            u32::mul_nbits(n as usize) == n as u64 * 32
        }
    }

    #[test]
    fn lg_nbits() {
        assert_eq!( u8::lg_nbits(), 3 );
        assert_eq!( u16::lg_nbits(), 4 );
        assert_eq!( u32::lg_nbits(), 5 );
        assert_eq!( u64::lg_nbits(), 6 );
    }

    #[test]
    fn low_mask() {
        assert_eq!(0b00011111, u8::low_mask(5));
        assert_eq!(0b0011111111111111, u16::low_mask(14));
        assert_eq!(0b1111111111111111, u16::low_mask(16));
    }

    #[test]
    fn nth_mask() {
        assert_eq!(0b10000000, u8::nth_mask(7));
        assert_eq!(0b01000000, u8::nth_mask(6));
        assert_eq!(0b00100000, u8::nth_mask(5));
        assert_eq!(0b00000010, u8::nth_mask(1));
        assert_eq!(0b00000001, u8::nth_mask(0));

        assert_eq!(0b0000000000000001, u16::nth_mask(0));
        assert_eq!(0b1000000000000000, u16::nth_mask(15));
    }

    #[test]
    fn get_bits() {
        assert_eq!(0b0,
                   0b0100110001110000u16.get_bits(0, 0));
        assert_eq!(0b010,
                   0b0100110001110000u16.get_bits(13, 3));
        assert_eq!(    0b110001,
                       0b0100110001110000u16.get_bits(6, 6));
        assert_eq!(           0b10000,
                              0b0100110001110000u16.get_bits(0, 5));
        assert_eq!(0b0100110001110000,
                   0b0100110001110000u16.get_bits(0, 16));
    }

    #[test]
    fn with_bits() {
        assert_eq!(0b0111111111000001,
                   0b0110001111000001u16.with_bits(10, 3, 0b111));
        assert_eq!(0b0101110111000001,
                   0b0110001111000001u16.with_bits(9, 5, 0b01110));
        assert_eq!(0b0110001111000001,
                   0b0110001111000001u16.with_bits(14, 0, 0b01110));
        assert_eq!(0b0110001110101010,
                   0b0110001111000001u16.with_bits(0, 8, 0b10101010));
        assert_eq!(0b0000000000000010,
                   0b0110001111000001u16.with_bits(0, 16, 0b10));
    }

    #[test]
    fn get_bit() {
        assert!(! 0b00000000u8.get_bit(0));
        assert!(! 0b00000000u8.get_bit(1));
        assert!(! 0b00000000u8.get_bit(2));
        assert!(! 0b00000000u8.get_bit(3));
        assert!(! 0b00000000u8.get_bit(7));
        assert!(! 0b10101010u8.get_bit(0));
        assert!(  0b10101010u8.get_bit(1));
        assert!(! 0b10101010u8.get_bit(2));
        assert!(  0b10101010u8.get_bit(3));
        assert!(  0b10101010u8.get_bit(7));
    }

    #[test]
    fn with_bit() {
        assert_eq!(0b00100000, 0b00000000u8.with_bit(5, true));
        assert_eq!(0b00000000, 0b00000000u8.with_bit(5, false));
        assert_eq!(0b10101010, 0b10101010u8.with_bit(7, true));
        assert_eq!(0b00101010, 0b10101010u8.with_bit(7, false));
        assert_eq!(0b10101011, 0b10101010u8.with_bit(0, true));
        assert_eq!(0b10101010, 0b10101010u8.with_bit(0, false));
    }

    #[test]
    fn floor_lg() {
        assert_eq!(0, 1u32.floor_lg());
        assert_eq!(1, 2u32.floor_lg());
        assert_eq!(1, 3u32.floor_lg());
        assert_eq!(2, 4u32.floor_lg());
        assert_eq!(2, 5u32.floor_lg());
        assert_eq!(2, 7u32.floor_lg());
        assert_eq!(3, 8u32.floor_lg());

        fn prop(n: u64) -> TestResult {
            if n == 0 { return TestResult::discard(); }

            TestResult::from_bool(
                2u64.pow(n.floor_lg() as u32) <= n
                    && 2u64.pow(n.floor_lg() as u32 + 1) > n)
        }

        quickcheck(prop as fn(u64) -> TestResult);
    }

    #[test]
    fn ceil_lg() {
        assert_eq!(0, 1u32.ceil_lg());
        assert_eq!(1, 2u32.ceil_lg());
        assert_eq!(2, 3u32.ceil_lg());
        assert_eq!(2, 4u32.ceil_lg());
        assert_eq!(3, 5u32.ceil_lg());
        assert_eq!(3, 7u32.ceil_lg());
        assert_eq!(3, 8u32.ceil_lg());
        assert_eq!(4, 9u32.ceil_lg());

        fn prop(n: u64) -> TestResult {
            if n <= 1 { return TestResult::discard(); }

            TestResult::from_bool(
                2u64.pow(n.ceil_lg() as u32) >= n
                    && 2u64.pow(n.ceil_lg() as u32 - 1) < n)
        }
        quickcheck(prop as fn(u64) -> TestResult);
    }

    #[test]
    fn block_bits() {
        assert_eq!( u16::block_bits(1, 0), 1 );
        assert_eq!( u16::block_bits(2, 0), 2 );
        assert_eq!( u16::block_bits(16, 0), 16 );
        assert_eq!( u16::block_bits(16, 1), 0 ); // boundary condition
        assert_eq!( u16::block_bits(23, 0), 16 );
        assert_eq!( u16::block_bits(23, 1), 7 );
        assert_eq!( u16::block_bits(35, 0), 16 );
        assert_eq!( u16::block_bits(35, 1), 16 );
        assert_eq!( u16::block_bits(35, 2), 3 );
        assert_eq!( u16::block_bits(48, 0), 16 );
        assert_eq!( u16::block_bits(48, 1), 16 );
        assert_eq!( u16::block_bits(48, 2), 16 );
        assert_eq!( u16::block_bits(48, 3), 0 ); // boundary condition
    }
}
