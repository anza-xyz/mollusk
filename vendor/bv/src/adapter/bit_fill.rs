use Bits;
use BlockType;
use iter::BlockIter;

use traits::get_masked_block;

/// Emulates a constant-valued bit-vector of a given size.
#[derive(Debug, Clone)]
pub struct BitFill<Block> {
    len: u64,
    block: Block,
}

impl<Block: BlockType> Bits for BitFill<Block> {
    type Block = Block;

    fn bit_len(&self) -> u64 {
        self.len
    }

    fn get_bit(&self, position: u64) -> bool {
        assert!(position < self.len,
                "BitFill::get_bit: out of bounds");
        self.block != Block::zero()
    }

    fn get_block(&self, position: usize) -> Self::Block {
        assert!(position < self.block_len(),
                "BitFill::get_block: out of bounds");
        get_masked_block(self, position)
    }

    fn get_raw_block(&self, position: usize) -> Self::Block {
        assert!(position < self.block_len(),
                "BitFill::get_raw_block: out of bounds");
        self.block
    }

    fn get_bits(&self, position: u64, len: usize) -> Self::Block {
        assert!(position + (len as u64) <= self.bit_len(),
                "BitFill::get_bits: out of bounds");
        self.block
    }
}

impl<Block: BlockType> BitFill<Block> {
    /// Constructs a compact bit-vector-like of `len` 0s.
    pub fn zeroes(len: u64) -> Self {
        BitFill {
            len,
            block: Block::zero(),
        }
    }

    /// Constructs a compact bit-vector-like of `len` 1s.
    pub fn ones(len: u64) -> Self {
        BitFill {
            len,
            block: !Block::zero(),
        }
    }
}

impl<T: Bits> PartialEq<T> for BitFill<T::Block> {
    fn eq(&self, other: &T) -> bool {
        BlockIter::new(self) == BlockIter::new(other)
    }
}

impl_index_from_bits! {
    impl[Block: BlockType] Index<u64> for BitFill<Block>;
}

impl_bit_sliceable_adapter! {
    impl[Block: BlockType] BitSliceable for BitFill<Block>;
    impl['a, Block: BlockType] BitSliceable for &'a BitFill<Block>;
}
