use BlockType;
use super::{Bits, BitsMut};

/// Extension trait for mutable operations on bit slices.
pub trait BitsMutExt: BitsMut {
    /// Assigns the bits of `other` to `self`.
    ///
    /// # Panics
    ///
    /// If `self.bit_len() != other.bit_len()`.
    fn bit_assign<T: Bits<Block = Self::Block>>(&mut self, other: T) {
        assert_eq!( self.bit_len(), other.bit_len(),
                    "BitsMutExt::bit_assign: arguments have different lengths" );

        let bit_len     = self.bit_len();
        let full_blocks = Self::Block::div_nbits(bit_len);
        let extra_bits  = Self::Block::mod_nbits(bit_len);

        for i in 0 .. full_blocks {
            let block = other.get_raw_block(i);
            self.set_block(i, block);
        }

        if extra_bits > 0 {
            let block = other.get_raw_block(full_blocks);
            self.set_bits(bit_len - extra_bits as u64, extra_bits, block);
        }
    }

    /// Assigns the bit-wise *and* of `self` and `other` to `self`.
    ///
    /// # Panics
    ///
    /// If `self.bit_len() != other.bit_len()`.
    fn bit_and_assign<T: Bits<Block = Self::Block>>(&mut self, other: T) {
        self.bit_zip_assign(other, |b1, b2| b1 & b2);
    }

    /// Assigns the bit-wise *or* of `self` and `other` to `self`.
    ///
    /// # Panics
    ///
    /// If `self.bit_len() != other.bit_len()`.
    fn bit_or_assign<T: Bits<Block = Self::Block>>(&mut self, other: T) {
        self.bit_zip_assign(other, |b1, b2| b1 | b2);
    }

    /// Assigns the bit-wise *xor* of `self` and `other` to `self`.
    ///
    /// # Panics
    ///
    /// If `self.bit_len() != other.bit_len()`.
    fn bit_xor_assign<T: Bits<Block = Self::Block>>(&mut self, other: T) {
        self.bit_zip_assign(other, |b1, b2| b1 ^ b2);
    }

    /// Performs an op-assignment from `other` to `self`.
    ///
    /// In particular, the given function is used to combine each
    /// block of `self` with a block of `other`, assigning the result
    /// back to `self`.
    ///
    /// # Panics
    ///
    /// If `self.bit_len() != other.bit_len()`.
    fn bit_zip_assign<T, F>(&mut self, other: T, mut fun: F)
        where T: Bits<Block = Self::Block>,
              F: FnMut(Self::Block, Self::Block) -> Self::Block {

        assert_eq!( self.bit_len(), other.bit_len(),
                    "BitsMutExt::bit_zip_assign: arguments have different lengths" );

        let bit_len     = self.bit_len();
        let full_blocks = Self::Block::div_nbits(bit_len);
        let extra_bits  = Self::Block::mod_nbits(bit_len);

        for i in 0 .. full_blocks {
            let self_block = self.get_raw_block(i);
            let other_block = other.get_raw_block(i);
            let combined_block = fun(self_block, other_block);
            self.set_block(i, combined_block);
        }

        if extra_bits > 0 {
            let self_block = self.get_block(full_blocks);
            let other_block = other.get_block(full_blocks);
            let combined_block = fun(self_block, other_block);
            self.set_bits(bit_len - extra_bits as u64, extra_bits, combined_block);
        }
    }
}

impl<T: BitsMut> BitsMutExt for T {}

#[cfg(test)]
mod test {
    use {BitVec, BitSliceable, BitSliceableMut};
    use super::*;

    #[test]
    fn bit_and_assign() {
        let mut bv1: BitVec = bit_vec![false, false, true, true];
        let     bv2: BitVec = bit_vec![false, true, true, false];

        bv1.bit_and_assign(&bv2);
        assert_eq!( bv1, bit_vec![false, false, true, false] );
    }

    #[test]
    #[should_panic]
    fn bit_and_assign_bad_sizes() {
        let mut bv1: BitVec = bit_vec![false, false, true, true, true];
        let     bv2: BitVec = bit_vec![false, true, true, false];

        bv1.bit_and_assign(&bv2);
    }

    #[test]
    fn bit_xor_assign_slice() {
        let mut v1 = vec![0b00_0011_11u8];
        let     v2 = [0b0_1010_101u8];

        v1.bit_slice_mut(2..6)
            .bit_xor_assign(v2.bit_slice(3..7));

        assert_eq!(v1, vec![0b00100111])
    }
}