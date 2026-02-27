use Bits;
use BlockType;
use iter::BlockIter;

/// The result of
/// [`BitsExt::bit_concat`](../trait.BitsExt.html#method.bit_concat).
///
/// The resulting bit vector adapter concatenates the bits of the two underlying
/// bit-vector-likes.
#[derive(Debug, Clone)]
pub struct BitConcat<T, U>(T, U);

impl<T, U> BitConcat<T, U> {
    pub (crate) fn new(bits1: T, bits2: U) -> Self {
        BitConcat(bits1, bits2)
    }
}

impl<T, U> Bits for BitConcat<T, U>
    where T: Bits,
          U: Bits<Block = T::Block> {

    type Block = T::Block;

    fn bit_len(&self) -> u64 {
        self.0.bit_len() + self.1.bit_len()
    }

    fn get_bit(&self, position: u64) -> bool {
        let len0 = self.0.bit_len();
        if position < len0 {
            self.0.get_bit(position)
        } else {
            self.1.get_bit(position - len0)
        }
    }

    fn get_block(&self, position: usize) -> Self::Block {
        let start_bit = Self::Block::mul_nbits(position);
        let count     = Self::Block::block_bits(self.bit_len(), position);
        let limit_bit = start_bit + count as u64;

        let len0 = self.0.bit_len();
        if limit_bit <= len0 {
            self.0.get_block(position)
        } else if start_bit < len0 {
            let block1 = self.0.get_raw_block(position);
            let block2 = self.1.get_raw_block(0);
            let size1  = (len0 - start_bit) as usize;
            let size2  = count - size1;
            block1.get_bits(0, size1) |
                (block2.get_bits(0, size2) << size1)
        } else {
            self.1.get_bits(start_bit - len0, count)
        }
    }
}

impl<T, U, V> PartialEq<V> for BitConcat<T, U>
    where T: Bits,
          U: Bits<Block = T::Block>,
          V: Bits<Block = T::Block> {

    fn eq(&self, other: &V) -> bool {
        BlockIter::new(self) == BlockIter::new(other)
    }
}

impl_index_from_bits! {
    impl[T: Bits, U: Bits<Block = T::Block>] Index<u64> for BitConcat<T, U>;
}

impl_bit_sliceable_adapter! {
    impl[T: Bits, U: Bits<Block = T::Block>] BitSliceable for BitConcat<T, U>;
    impl['a, T: Bits, U: Bits<Block = T::Block>] BitSliceable for &'a BitConcat<T, U>;
}

