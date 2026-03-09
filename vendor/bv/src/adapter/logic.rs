use {Bits, BitSliceable};
use BlockType;
use iter::BlockIter;

use traits::get_masked_block;

use std::cmp;

/// The result of [`BitsExt::bit_not`](../trait.BitsExt.html#method.bit_not).
///
/// The resulting bit vector adapter *not*s the bits of the underlying
/// bit-vector-like.
#[derive(Clone, Debug)]
pub struct BitNot<T>(T);

impl<T> BitNot<T> {
    pub (crate) fn new(bits: T) -> Self {
        BitNot(bits)
    }
}

/// The result of [`BitsExt::bit_and`](../trait.BitsExt.html#method.bit_and).
///
/// The resulting bit vector adapter *and*s the bits of the two underlying
/// bit-vector-likes.
#[derive(Clone, Debug)]
pub struct BitAnd<T, U>(BitBinOp<T, U>);

impl<T: Bits, U: Bits<Block = T::Block>> BitAnd<T, U> {
    pub (crate) fn new(bits1: T, bits2: U) -> Self {
        BitAnd(BitBinOp::new(bits1, bits2))
    }
}

/// The result of [`BitsExt::bit_or`](../trait.BitsExt.html#method.bit_or).
///
/// The resulting bit vector adapter *or*s the bits of the two underlying
/// bit-vector-likes.
#[derive(Clone, Debug)]
pub struct BitOr<T, U>(BitBinOp<T, U>);

impl<T: Bits, U: Bits<Block = T::Block>> BitOr<T, U> {
    pub (crate) fn new(bits1: T, bits2: U) -> Self {
        BitOr(BitBinOp::new(bits1, bits2))
    }
}

/// The result of [`BitsExt::bit_xor`](../trait.BitsExt.html#method.bit_xor).
///
/// The resulting bit vector adapter *xor*s the bits of the two underlying
/// bit-vector-likes.
#[derive(Clone, Debug)]
pub struct BitXor<T, U>(BitBinOp<T, U>);

impl<T: Bits, U: Bits<Block = T::Block>> BitXor<T, U> {
    pub (crate) fn new(bits1: T, bits2: U) -> Self {
        BitXor(BitBinOp::new(bits1, bits2))
    }
}

/// The result of [`BitsExt::bit_zip`](../trait.BitsExt.html#method.bit_zip).
#[derive(Clone, Debug)]
pub struct BitZip<T, U, F> {
    ops: BitBinOp<T, U>,
    fun: F,
}

impl<T: Bits, U: Bits<Block = T::Block>, F> BitZip<T, U, F> {
    pub (crate) fn new(bits1: T, bits2: U, fun: F) -> Self {
        BitZip {
            ops: BitBinOp::new(bits1, bits2),
            fun,
        }
    }
}

/// Used to store the two operands to a bitwise logical operation on
/// `Bits`es, along with the length of the result (min the length of
/// the operands). (Note that `len` is derivable from `op1` and `op2`,
/// but it probably makes sense to cache it.)
#[derive(Clone, Debug)]
struct BitBinOp<T, U> {
    op1: T,
    op2: U,
    len: u64,
}

impl<T: Bits, U: Bits<Block = T::Block>> BitBinOp<T, U> {
    fn new(op1: T, op2: U) -> Self {
        let len = cmp::min(op1.bit_len(), op2.bit_len());
        BitBinOp { op1, op2, len, }
    }

    fn bit1(&self, position: u64) -> bool {
        self.op1.get_bit(position)
    }

    fn bit2(&self, position: u64) -> bool {
        self.op2.get_bit(position)
    }

    fn block1(&self, position: usize) -> T::Block {
        self.op1.get_raw_block(position)
    }

    fn block2(&self, position: usize) -> T::Block {
        self.op2.get_raw_block(position)
    }
}

impl<T: Bits> Bits for BitNot<T> {
    type Block = T::Block;

    fn bit_len(&self) -> u64 {
        self.0.bit_len()
    }

    fn get_bit(&self, position: u64) -> bool {
        !self.0.get_bit(position)
    }

    fn get_block(&self, position: usize) -> Self::Block {
        !self.0.get_block(position)
    }

    fn get_raw_block(&self, position: usize) -> Self::Block {
        !self.0.get_raw_block(position)
    }
}

impl_index_from_bits! {
    impl[T: Bits] Index<u64> for BitNot<T>;
}

impl<R, T> BitSliceable<R> for BitNot<T>
    where T: BitSliceable<R> {

    type Slice = BitNot<T::Slice>;

    fn bit_slice(self, range: R) -> Self::Slice {
        BitNot(self.0.bit_slice(range))
    }
}

impl_bit_sliceable_adapter! {
    impl['a, T: Bits] BitSliceable for &'a BitNot<T>;
}

impl<T, U> PartialEq<U> for BitNot<T>
    where T: Bits,
          U: Bits<Block = T::Block> {

    fn eq(&self, other: &U) -> bool {
        BlockIter::new(self) == BlockIter::new(other)
    }
}

macro_rules! impl_bits_bin_op {
    ( $target:ident as $block_op:tt $bool_op:tt ) => {
        impl<T, U> Bits for $target<T, U>
            where T: Bits,
                  U: Bits<Block = T::Block>
        {
            type Block = T::Block;

            fn bit_len(&self) -> u64 {
                self.0.len
            }

            fn get_bit(&self, position: u64) -> bool {
                assert!( position < self.bit_len(),
                         format!("{}::get_bit: out of bounds", stringify!($target)) );
                self.0.bit1(position) $bool_op self.0.bit2(position)
            }

            fn get_block(&self, position: usize) -> Self::Block {
                assert!( position < self.block_len(),
                         format!("{}::get_block: out of bounds", stringify!($target)) );
                get_masked_block(self, position)
            }

            fn get_raw_block(&self, position: usize) -> Self::Block {
                self.0.block1(position) $block_op self.0.block2(position)
            }
        }

        impl_index_from_bits! {
            impl[T: Bits, U: Bits<Block = T::Block>] Index<u64> for $target<T, U>;
        }

        impl<Block, R, T, U> BitSliceable<R> for $target<T, U>
            where Block: BlockType,
                  R: Clone,
                  T: BitSliceable<R, Block = Block>,
                  U: BitSliceable<R, Block = Block> {

            type Slice = $target<T::Slice, U::Slice>;

            fn bit_slice(self, range: R) -> Self::Slice {
                $target(BitBinOp::new(self.0.op1.bit_slice(range.clone()),
                                      self.0.op2.bit_slice(range)))
            }
        }

        impl_bit_sliceable_adapter! {
            impl['a, T: Bits, U: Bits<Block = T::Block>] BitSliceable for &'a $target<T, U>;
        }

        impl<T, U, V> PartialEq<V> for $target<T, U>
            where T: Bits,
                  U: Bits<Block = T::Block>,
                  V: Bits<Block = T::Block> {

            fn eq(&self, other: &V) -> bool {
                BlockIter::new(self) == BlockIter::new(other)
            }
        }
    };
}

impl_bits_bin_op!(BitAnd as & &&);
impl_bits_bin_op!(BitOr  as | ||);
impl_bits_bin_op!(BitXor as ^ ^);

impl<T, U, F> Bits for BitZip<T, U, F>
    where T: Bits,
          U: Bits<Block = T::Block>,
          F: Fn(T::Block, T::Block) -> T::Block {
    type Block = T::Block;

    fn bit_len(&self) -> u64 {
        self.ops.len
    }

    fn get_block(&self, position: usize) -> Self::Block {
        assert!( position < self.block_len(), "BitZip::get_block: out of bounds" );
        get_masked_block(self, position)
    }

    fn get_raw_block(&self, position: usize) -> Self::Block {
        (self.fun)(self.ops.block1(position), self.ops.block2(position))
    }
}

impl_index_from_bits! {
    impl[T: Bits, U: Bits<Block = T::Block>,
         F: Fn(T::Block, T::Block) -> T::Block]
        Index<u64> for BitZip<T, U, F>;
}

impl<Block, R, T, U, F> BitSliceable<R> for BitZip<T, U, F>
    where Block: BlockType,
          R: Clone,
          T: BitSliceable<R, Block = Block>,
          U: BitSliceable<R, Block = Block>,
          F: Fn(Block, Block) -> Block {

    type Slice = BitZip<T::Slice, U::Slice, F>;

    fn bit_slice(self, range: R) -> Self::Slice {
        BitZip {
            ops: BitBinOp::new(self.ops.op1.bit_slice(range.clone()),
                               self.ops.op2.bit_slice(range)),
            fun: self.fun,
        }
    }
}

impl_bit_sliceable_adapter! {
    impl['a, T: Bits, U: Bits<Block = T::Block>,
         F: Fn(T::Block, T::Block) -> T::Block]
        BitSliceable for &'a BitZip<T, U, F>;
}

impl<T, U, F, V> PartialEq<V> for BitZip<T, U, F>
    where T: Bits,
          U: Bits<Block = T::Block>,
          V: Bits<Block = T::Block>,
          F: Fn(T::Block, T::Block) -> T::Block {

    fn eq(&self, other: &V) -> bool {
        BlockIter::new(self) == BlockIter::new(other)
    }
}

