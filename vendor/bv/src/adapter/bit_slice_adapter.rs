use {Bits, BitsMut, BitSliceable, BlockType};
use iter::BlockIter;

use range_compat::*;

/// An adapter that turns any implementation of `Bits` into a slice.
///
/// This is likely less efficient than [`BitSlice`].
///
/// [`BitSlice`]: ../struct.BitSlice.html
#[derive(Copy, Clone, Debug)]
pub struct BitSliceAdapter<T> {
    bits:  T,
    start: u64,
    len:   u64,
}

impl<T: Bits> BitSliceAdapter<T> {
    /// Creates a new slice adaptor from the given bit-vector-like.
    ///
    /// Takes the index of the start bit, and the length to slice.
    ///
    /// # Panics
    ///
    /// Out of bounds if `start + len > bits.bit_len()`.
    pub fn new(bits: T, start: u64, len: u64) -> Self {
        assert!( start + len <= bits.bit_len(),
                 "BitSliceAdapter::new: out of bounds");
        BitSliceAdapter { bits, start, len }
    }

    /// Reslices an existing slice adapter, by value.
    ///
    /// Takes the index of the start bit, relative to the indexing
    /// of the adapter.
    ///
    /// # Panics
    ///
    /// Out of bounds if `start + len > self.bit_len()`.
    fn reslice(self, start: u64, len: u64) -> Self {
        assert!( start + len <= self.bit_len(),
                 "BitSliceAdapter::reslice: out of bounds" );
        BitSliceAdapter {
            bits:  self.bits,
            start: self.start + start,
            len,
        }
    }

    /// Reslices an existing slice adapter, by reference.
    ///
    /// Takes the index of the start bit, relative to the indexing
    /// of the adapter.
    ///
    /// # Panics
    ///
    /// Out of bounds if `start + len > self.bit_len()`.
    fn reslice_ref(&self, start: u64, len: u64) -> BitSliceAdapter<&T> {
        assert!( start + len <= self.bit_len(),
                 "BitSliceAdapter::reslice: out of bounds" );
        BitSliceAdapter {
            bits:  &self.bits,
            start: self.start + start,
            len,
        }
    }
}

impl<T, U> PartialEq<U> for BitSliceAdapter<T>
    where T: Bits,
          U: Bits<Block = T::Block> {

    fn eq(&self, other: &U) -> bool {
        BlockIter::new(self) == BlockIter::new(other)
    }
}

macro_rules! impl_bit_sliceable_adapter {
    (
        $(
            impl[ $($param:tt)* ] BitSliceable for $target:ty ;
        )+
    ) => {
        $(
            impl<$($param)*> ::BitSliceable<::std::ops::Range<u64>> for $target {
                type Slice = ::adapter::BitSliceAdapter<Self>;

                fn bit_slice(self, range: ::std::ops::Range<u64>) -> Self::Slice {
                    assert!( range.start <= range.end,
                             format!("{}::slice: bad range", stringify!($target)) );
                    ::adapter::BitSliceAdapter::new(self, range.start, range.end - range.start)
                }
            }

            impl<$($param)*> ::BitSliceable<::std::ops::RangeFrom<u64>> for $target {
                type Slice = ::adapter::BitSliceAdapter<Self>;

                fn bit_slice(self, range: ::std::ops::RangeFrom<u64>) -> Self::Slice {
                    let len = self.bit_len();
                    self.bit_slice(range.start .. len)
                }
            }

            impl<$($param)*> ::BitSliceable<::std::ops::RangeTo<u64>> for $target {
                type Slice = ::adapter::BitSliceAdapter<Self>;

                fn bit_slice(self, range: ::std::ops::RangeTo<u64>) -> Self::Slice {
                    ::adapter::BitSliceAdapter::new(self, 0, range.end)
                }
            }

            impl<$($param)*> ::BitSliceable<::std::ops::RangeFull> for $target {
                type Slice = ::adapter::BitSliceAdapter<Self>;

                fn bit_slice(self, _range: ::std::ops::RangeFull) -> Self::Slice {
                    let len = self.bit_len();
                    ::adapter::BitSliceAdapter::new(self, 0, len)
                }
            }

            #[cfg(inclusive_range)]
            impl<$($param)*> ::BitSliceable<::std::ops::RangeInclusive<u64>> for $target {
                type Slice = ::adapter::BitSliceAdapter<Self>;

                fn bit_slice(self, range: ::std::ops::RangeInclusive<u64>) -> Self::Slice {
                    let (start, end) = ::range_compat::get_inclusive_bounds(range)
                        .expect("BitSliceable::bit_slice: bad inclusive range");
                    ::adapter::BitSliceAdapter::new(self, start, end - start + 1)
                }
            }

            #[cfg(inclusive_range)]
            impl<$($param)*> ::BitSliceable<::std::ops::RangeToInclusive<u64>> for $target {
                type Slice = ::adapter::BitSliceAdapter<Self>;

                fn bit_slice(self, range: ::std::ops::RangeToInclusive<u64>) -> Self::Slice {
                    ::adapter::BitSliceAdapter::new(self, 0, range.end + 1)
                }
            }
        )+
    };
}

impl<T: Bits> Bits for BitSliceAdapter<T> {
    type Block = T::Block;

    fn bit_len(&self) -> u64 {
        self.len
    }

    fn get_bit(&self, position: u64) -> bool {
        assert!( position < self.bit_len(),
                 "BitSliceAdapter::get_bit: out of bounds" );
        self.bits.get_bit(self.start + position)
    }

    fn get_block(&self, position: usize) -> Self::Block {
        assert!( position < self.block_len(),
                 "BitSliceAdapter::get_block: out of bounds" );
        let (real_start, real_len) =
            get_block_addr::<T::Block>(self.start, self.len, position);
        self.bits.get_bits(real_start, real_len)
    }

    fn get_bits(&self, start: u64, count: usize) -> Self::Block {
        assert!( start + count as u64 <= self.bit_len(),
                 "BitSliceAdapter::get_bits: out of bounds" );
        self.bits.get_bits(self.start + start, count)
    }
}

impl<T: BitsMut> BitsMut for BitSliceAdapter<T> {
    fn set_bit(&mut self, position: u64, value: bool) {
        assert!( position < self.bit_len(),
                 "BitSliceAdapter::set_bit: out of bounds" );
        self.bits.set_bit(self.start + position, value);
    }

    fn set_block(&mut self, position: usize, value: Self::Block) {
        assert!( position < self.block_len(),
                 "BitSliceAdapter::get_block: out of bounds" );
        let (real_start, real_len) =
            get_block_addr::<T::Block>(self.start, self.len, position);
        self.bits.set_bits(real_start, real_len, value);
    }

    fn set_bits(&mut self, start: u64, count: usize, value: Self::Block) {
        assert!( start + count as u64 <= self.bit_len(),
                 "BitSliceAdapter::set_bits: out of bounds" );
        self.bits.set_bits(self.start + start, count, value);
    }
}

// For a slice starting at `start`, of length `len`, finds the parameters
// for extracting the `position`th block. The parameters are the bit position
// of the start of the block, and the number of bits in the block.
fn get_block_addr<Block: BlockType>(start: u64, len: u64, position: usize)
                                    -> (u64, usize) {

    let real_start = start + Block::mul_nbits(position);
    let block_size = Block::nbits() as u64;
    let real_len   = if real_start + block_size < start + len {
        block_size
    } else {
        (start + len - real_start)
    };

    (real_start, real_len as usize)
}

impl_index_from_bits! {
    impl[T: Bits] Index<u64> for BitSliceAdapter<T>;
}

impl<T: Bits> BitSliceable<Range<u64>> for BitSliceAdapter<T> {
    type Slice = Self;

    fn bit_slice(self, range: Range<u64>) -> Self::Slice {
        assert!( range.start <= range.end,
                 "BitSliceAdapter::bit_slice: bad range" );
        self.reslice(range.start, range.end - range.start)
    }
}

impl<T: Bits> BitSliceable<RangeTo<u64>> for BitSliceAdapter<T> {
    type Slice = Self;

    fn bit_slice(self, range: RangeTo<u64>) -> Self::Slice {
        self.reslice(0, range.end)
    }
}

impl<T: Bits> BitSliceable<RangeFrom<u64>> for BitSliceAdapter<T> {
    type Slice = Self;

    fn bit_slice(self, range: RangeFrom<u64>) -> Self::Slice {
        let len = self.bit_len();
        assert!( range.start <= len,
                 "BitSliceAdapter::bit_slice: out of bounds" );
        self.reslice(range.start, len - range.start)
    }
}

impl<T: Bits> BitSliceable<RangeFull> for BitSliceAdapter<T> {
    type Slice = Self;

    fn bit_slice(self, _range: RangeFull) -> Self::Slice {
        self
    }
}

#[cfg(inclusive_range)]
impl<T: Bits> BitSliceable<RangeInclusive<u64>> for BitSliceAdapter<T> {
    type Slice = Self;

    fn bit_slice(self, range: RangeInclusive<u64>) -> Self::Slice {
        let (start, limit) = get_inclusive_bounds(range)
            .expect("BitSliceAdapter::bit_slice: bad range");
        self.reslice(start, limit - start + 1)
    }
}

#[cfg(inclusive_range)]
impl<T: Bits> BitSliceable<RangeToInclusive<u64>> for BitSliceAdapter<T> {
    type Slice = Self;

    fn bit_slice(self, range: RangeToInclusive<u64>) -> Self::Slice {
        self.reslice(0, range.end + 1)
    }
}

impl<'a, T: Bits> BitSliceable<Range<u64>> for &'a BitSliceAdapter<T> {
    type Slice = BitSliceAdapter<&'a T>;

    fn bit_slice(self, range: Range<u64>) -> Self::Slice {
        assert!( range.start <= range.end,
                 "BitSliceAdapter::bit_slice: bad range" );
        self.reslice_ref(range.start, range.end - range.start)
    }
}

impl<'a, T: Bits> BitSliceable<RangeTo<u64>> for &'a BitSliceAdapter<T> {
    type Slice = BitSliceAdapter<&'a T>;

    fn bit_slice(self, range: RangeTo<u64>) -> Self::Slice {
        self.reslice_ref(0, range.end)
    }
}

impl<'a, T: Bits> BitSliceable<RangeFrom<u64>> for &'a BitSliceAdapter<T> {
    type Slice = BitSliceAdapter<&'a T>;

    fn bit_slice(self, range: RangeFrom<u64>) -> Self::Slice {
        let len = self.bit_len();
        assert!( range.start <= len,
                 "BitSliceAdapter::bit_slice: out of bounds" );
        self.reslice_ref(range.start, len - range.start)
    }
}

impl<'a, T: Bits> BitSliceable<RangeFull> for &'a BitSliceAdapter<T> {
    type Slice = BitSliceAdapter<&'a T>;

    fn bit_slice(self, _range: RangeFull) -> Self::Slice {
        self.reslice_ref(0, self.bit_len())
    }
}

#[cfg(inclusive_range)]
impl<'a, T: Bits> BitSliceable<RangeInclusive<u64>> for &'a BitSliceAdapter<T> {
    type Slice = BitSliceAdapter<&'a T>;

    fn bit_slice(self, range: RangeInclusive<u64>) -> Self::Slice {
        let (start, limit) = get_inclusive_bounds(range)
            .expect("BitSliceAdapter::bit_slice: bad range");
        self.reslice_ref(start, limit - start + 1)
    }
}

#[cfg(inclusive_range)]
impl<'a, T: Bits> BitSliceable<RangeToInclusive<u64>> for &'a BitSliceAdapter<T> {
    type Slice = BitSliceAdapter<&'a T>;

    fn bit_slice(self, range: RangeToInclusive<u64>) -> Self::Slice {
        self.reslice_ref(0, range.end + 1)
    }
}

