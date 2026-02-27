use Bits;
use BlockType;

use std::cmp::Ordering;

/// An iterator over the blocks of a bit-vector-like.
#[derive(Clone, Debug)]
pub struct BlockIter<T> {
    bits: T,
    pos:  usize,
}
// Invariant: pos <= bits.block_len()

impl<T: Bits> BlockIter<T> {
    /// Creates a new block iterator from a `Bits` instance.
    pub fn new(bits: T) -> Self {
        BlockIter { bits, pos:  0, }
    }

    /// Returns the number of *bits* remaining in the iterator.
    pub fn bit_len(&self) -> u64 {
        self.bits.bit_len() - T::Block::mul_nbits(self.pos)
    }
}

impl<T: Bits> Iterator for BlockIter<T> {
    type Item = T::Block;

    fn next(&mut self) -> Option<T::Block> {
        if self.pos < self.bits.block_len() {
            let result = Some(self.bits.get_block(self.pos));
            self.pos += 1;
            result
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<T: Bits> ExactSizeIterator for BlockIter<T> {
    fn len(&self) -> usize {
        self.bits.block_len() - self.pos
    }
}

fn cmp_block_iter<T, U>(iter1: &BlockIter<T>, iter2: &BlockIter<U>) -> Ordering
    where T: Bits,
          U: Bits<Block = T::Block> {

    let len1    = iter1.bit_len();
    let len2    = iter2.bit_len();
    let len_cmp = len1.cmp(&len2);
    if len_cmp != Ordering::Equal { return len_cmp; }

    for i in 0 .. iter1.len() {
        let block1    = iter1.bits.get_block(iter1.pos + i);
        let block2    = iter2.bits.get_block(iter2.pos + i);
        let block_cmp = block1.cmp(&block2);
        if block_cmp != Ordering::Equal { return block_cmp; }
    }

    return Ordering::Equal;
}

impl<T, U> PartialEq<BlockIter<U>> for BlockIter<T>
    where T: Bits,
          U: Bits<Block = T::Block> {

    fn eq(&self, other: &BlockIter<U>) -> bool {
        cmp_block_iter(self, other) == Ordering::Equal
    }
}

impl<T: Bits> Eq for BlockIter<T> {}

impl<T, U> PartialOrd<BlockIter<U>> for BlockIter<T>
    where T: Bits,
          U: Bits<Block = T::Block> {

    fn partial_cmp(&self, other: &BlockIter<U>) -> Option<Ordering> {
        Some(cmp_block_iter(self, other))
    }
}

impl<T: Bits> Ord for BlockIter<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_block_iter(self, other)
    }
}

#[cfg(test)]
mod test {
    use BitVec;
    use super::*;

    fn eq_iter<T, U>(bits1: T, bits2: U) -> bool
        where T: Bits,
              U: Bits<Block = T::Block> {

        let i1 = BlockIter::new(bits1);
        let i2 = BlockIter::new(bits2);
        i1 == i2
    }

    #[test]
    fn empty() {
        let bv1: BitVec = bit_vec![];
        let bv2: BitVec = bit_vec![];
        assert!( eq_iter(&bv1, &bv2) );
    }

    #[test]
    fn same() {
        let bv1: BitVec = bit_vec![true, false, true];
        let bv2: BitVec = bit_vec![true, false, true];
        assert!( eq_iter(&bv1, &bv2) );
    }

    #[test]
    fn different() {
        let bv1: BitVec = bit_vec![true, false, false];
        let bv2: BitVec = bit_vec![true, false, true];
        assert!( !eq_iter(&bv1, &bv2) );
    }

    #[test]
    fn different_lengths() {
        let bv1: BitVec = bit_vec![true, false, true, false];
        let bv2: BitVec = bit_vec![true, false, true];
        assert!( !eq_iter(&bv1, &bv2) );
    }
}
