use super::Bits;
use storage::{BlockType, Address};

/// Mutable bit vector operations that don’t affect the length.
///
/// Minimal complete definition is `set_bit` or `set_block`, since each
/// is defined in terms of the other. Note that `set_block` in terms of
/// `set_bit` is inefficient, and thus you should implement `set_block`
/// directly if possible.
pub trait BitsMut: Bits {
    /// Sets the bit at `position` to `value`.
    ///
    /// The default implementation uses `get_raw_block` and `set_block`.
    ///
    /// # Panics
    ///
    /// Panics if `position` is out of bounds.
    fn set_bit(&mut self, position: u64, value: bool) {
        assert!(position < self.bit_len(), "BitsMut::set_bit: out of bounds");

        let address = Address::new::<Self::Block>(position);
        let old_block = self.get_raw_block(address.block_index);
        let new_block = old_block.with_bit(address.bit_offset, value);
        self.set_block(address.block_index, new_block);
    }

    /// Sets the block at `position` to `value`.
    ///
    /// The bits are laid out `Block::nbits()` per block, with the notional
    /// zeroth bit in the least significant position. If `self.bit_len()` is
    /// not a multiple of `Block::nbits()` then the last block will
    /// contain extra bits that are not part of the bit vector. Implementations
    /// of `set_block` should not change those trailing bits.
    ///
    /// The default implementation sets a block by setting each of its bits
    /// in turn. Consider it a slow reference implementation, and override it.
    ///
    /// # Panics
    ///
    /// Panics if `position` is out of bounds.
    fn set_block(&mut self, position: usize, mut value: Self::Block) {
        let offset = Self::Block::mul_nbits(position);
        let count  = Self::Block::block_bits(self.bit_len(), position);

        for i in 0 .. count as u64 {
            let bit = value & Self::Block::one() != Self::Block::zero();
            self.set_bit(offset + i, bit);
            value = value >> 1;
        }
    }

    /// Sets `count` bits starting at bit index `start`, interpreted as a
    /// little-endian integer.
    ///
    /// # Panics
    ///
    /// Panics if the bit span goes out of bounds.
    fn set_bits(&mut self, start: u64, count: usize, value: Self::Block) {
        let limit = start + count as u64;
        assert!(limit <= self.bit_len(), "BitsMut::set_bits: out of bounds");

        let address = Address::new::<Self::Block>(start);
        let margin = Self::Block::nbits() - address.bit_offset;

        if margin >= count {
            let old_block = self.get_raw_block(address.block_index);
            let new_block = old_block.with_bits(address.bit_offset, count, value);
            self.set_block(address.block_index, new_block);
            return;
        }

        let extra = count - margin;

        let old_block1 = self.get_raw_block(address.block_index);
        let old_block2 = self.get_raw_block(address.block_index + 1);

        let high_bits = value >> margin;

        let new_block1 = old_block1.with_bits(address.bit_offset,
                                              margin, value);
        let new_block2 = old_block2.with_bits(0, extra, high_bits);
        self.set_block(address.block_index, new_block1);
        self.set_block(address.block_index + 1, new_block2);
    }
}

impl<'a, T: BitsMut + ?Sized> BitsMut for &'a mut T {
    fn set_bit(&mut self, position: u64, value: bool) {
        T::set_bit(*self, position, value);
    }

    fn set_block(&mut self, position: usize, value: Self::Block) {
        T::set_block(*self, position, value);
    }

    fn set_bits(&mut self, start: u64, count: usize, value: Self::Block) {
        T::set_bits(*self, start, count, value);
    }
}

impl<Block: BlockType> BitsMut for Box<dyn BitsMut<Block = Block>> {
    fn set_bit(&mut self, position: u64, value: bool) {
        (**self).set_bit(position, value);
    }

    fn set_block(&mut self, position: usize, value: Block) {
        (**self).set_block(position, value);
    }

    fn set_bits(&mut self, start: u64, len: usize, value: Block) {
        (**self).set_bits(start, len, value);
    }
}

impl<Block: BlockType> BitsMut for [Block] {
    fn set_bit(&mut self, position: u64, value: bool) {
        let address = Address::new::<Block>(position);
        let block = &mut self[address.block_index];
        *block = block.with_bit(address.bit_offset, value);
    }

    fn set_block(&mut self, position: usize, value: Block) {
        self[position] = value;
    }
}

impl<Block: BlockType> BitsMut for Vec<Block> {
    fn set_bit(&mut self, position: u64, value: bool) {
        <[Block]>::set_bit(&mut *self, position, value);
    }

    fn set_block(&mut self, position: usize, value: Block) {
        <[Block]>::set_block(&mut *self, position, value);
    }
}

impl BitsMut for [bool] {
    fn set_bit(&mut self, position: u64, value: bool) {
        let position = position.to_usize()
            .expect("Vec<bool>::set_bit: overflow");
        self[position] = value;
    }
}

impl BitsMut for Vec<bool> {
    #[inline]
    fn set_bit(&mut self, position: u64, value: bool) {
        self.as_mut_slice().set_bit(position, value)
    }
}

