use super::Bits;
use adapter::*;

/// Extension trait for adapter operations on bit slices.
///
/// The methods return lazy adapter objects that query the underlying bit vectors
/// and perform operations as needed. To eagerly evaluate a result, copy
/// it into a vector using the [`Bits::to_bit_vec`] method, as in the example below.
///
/// This trait is currently `pub use`d from the [`adapter`] module, but that alias
/// is deprecated.
///
/// [`Bits::to_bit_vec`]: trait.Bits.html#method.to_bit_vec
/// [`adapter`]: adapter/index.html
///
/// # Examples
///
/// ```
/// use bv::*;
///
/// let bv1: BitVec = bit_vec![false, false, true, true];
/// let bv2: BitVec = bit_vec![false, true, false, true];
///
/// let and_bv = bv1.bit_and(&bv2);
///
/// assert_eq!( and_bv[0], false );
/// assert_eq!( and_bv[1], false );
/// assert_eq!( and_bv[2], false );
/// assert_eq!( and_bv[3], true );
///
/// let bv3 = and_bv.to_bit_vec();
/// assert_eq!( bv3, bit_vec![false, false, false, true] );
/// ```
pub trait BitsExt: Bits {

    /// Concatenates two bit vectors, with the bits of `self` followed by the bits
    /// of `other`.
    fn bit_concat<Other>(&self, other: Other) -> BitConcat<&Self, Other>
        where Other: Bits<Block = Self::Block> {

        BitConcat::new(self, other)
    }

    /// Concatenates two bit vectors, with the bits of `self` followed by the bits
    /// of `other`.
    ///
    /// Consumes `self`.
    fn into_bit_concat<Other>(self, other: Other) -> BitConcat<Self, Other>
        where Self: Sized,
              Other: Bits<Block = Self::Block> {

        BitConcat::new(self, other)
    }

    /// Pads `self` with 0s on the right to reach at least `len` bits in length.
    ///
    /// If `self` is already long enough, the length is unchanged.
    fn bit_pad(&self, len: u64) -> BitConcat<&Self, BitFill<Self::Block>> {
        self.into_bit_pad(len)
    }

    /// Pads `self` with 0s on the right to reach at least `len` bits in length.
    ///
    /// If `self` is already long enough, the length is unchanged.
    ///
    /// Consumes `self`.
    fn into_bit_pad(self, len: u64) -> BitConcat<Self, BitFill<Self::Block>>
        where Self: Sized {

        let have = self.bit_len();
        let need = if len > have {len - have} else {0};
        self.into_bit_concat(BitFill::zeroes(need))
    }

    /// Returns an object that inverts the values of all the bits in `self`.
    fn bit_not(&self) -> BitNot<&Self> {
        BitNot::new(self)
    }

    /// Returns an object that inverts the values of all the bits in `self`.
    ///
    /// Consumes `self`.
    fn into_bit_not(self) -> BitNot<Self>
        where Self: Sized
    {
        BitNot::new(self)
    }

    /// Returns an object that lazily computes the bit-wise conjunction
    /// of two bit-vector-likes.
    ///
    /// If the lengths of the operands differ, the result will have
    /// the minimum of the two.
    fn bit_and<Other>(&self, other: Other) -> BitAnd<&Self, Other>
        where Other: Bits<Block = Self::Block> {

        BitAnd::new(self, other)
    }

    /// Returns an object that lazily computes the bit-wise conjunction
    /// of two bit-vector-likes.
    ///
    /// If the lengths of the operands differ, the result will have
    /// the minimum of the two.
    ///
    /// Consumes `self`.
    fn into_bit_and<Other>(self, other: Other) -> BitAnd<Self, Other>
        where Self: Sized,
              Other: Bits<Block = Self::Block> {
        BitAnd::new(self, other)
    }

    /// Returns an object that lazily computes the bit-wise disjunction
    /// of two bit-vector-likes.
    ///
    /// If the lengths of the operands differ, the result will have
    /// the minimum of the two.
    fn bit_or<Other>(&self, other: Other) -> BitOr<&Self, Other>
        where Other: Bits<Block = Self::Block> {

        BitOr::new(self, other)
    }

    /// Returns an object that lazily computes the bit-wise disjunction
    /// of two bit-vector-likes.
    ///
    /// If the lengths of the operands differ, the result will have
    /// the minimum of the two.
    ///
    /// Consumes `self`.
    fn into_bit_or<Other>(self, other: Other) -> BitOr<Self, Other>
        where Self: Sized,
              Other: Bits<Block = Self::Block> {

        BitOr::new(self, other)
    }

    /// Returns an object that lazily computes the bit-wise xor of two
    /// bit-vector-likes.
    ///
    /// If the lengths of the operands differ, the result will have
    /// the minimum of the two.
    fn bit_xor<Other>(&self, other: Other) -> BitXor<&Self, Other>
        where Other: Bits<Block = Self::Block> {

        BitXor::new(self, other)
    }

    /// Returns an object that lazily computes the bit-wise xor of two
    /// bit-vector-likes.
    ///
    /// If the lengths of the operands differ, the result will have
    /// the minimum of the two.
    ///
    /// Consumes `self`.
    fn into_bit_xor<Other>(self, other: Other) -> BitXor<Self, Other>
        where Self: Sized,
              Other: Bits<Block = Self::Block> {

        BitXor::new(self, other)
    }

    /// Returns an object that lazily zips a function over the blocks of
    /// two bit-vector-like.
    ///
    /// The third parameter to the zipping function `fun` is the number of
    /// bits in the block currently being processed. (This will be
    /// `Self::Block::nbits()` for all but the last block.)
    ///
    /// If the lengths of the operands differ, the result will have
    /// the minimum of the two.
    fn bit_zip<Other, F>(&self, other: Other, fun: F) -> BitZip<&Self, Other, F>
        where Other: Bits<Block = Self::Block>,
              F: Fn(Self::Block, Self::Block, usize) -> Self::Block {

        BitZip::new(self, other, fun)
    }

    /// Returns an object that lazily zips a function over the blocks of
    /// two bit-vector-like.
    ///
    /// The third parameter to the zipping function `fun` is the number of
    /// bits in the block currently being processed. (This will be
    /// `Self::Block::nbits()` for all but the last block.)
    ///
    /// If the lengths of the operands differ, the result will have
    /// the minimum of the two.
    ///
    /// Consumes `self`.
    fn into_bit_zip<Other, F>(self, other: Other, fun: F) -> BitZip<Self, Other, F>
        where Self: Sized,
              Other: Bits<Block = Self::Block>,
              F: Fn(Self::Block, Self::Block, usize) -> Self::Block {

        BitZip::new(self, other, fun)
    }
}

impl<T: Bits> BitsExt for T {}

