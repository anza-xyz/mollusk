use iter::BlockIter;
use traits::{Bits, BitsMut, BitSliceable, get_masked_block};
use storage::{Address, BlockType};
use range_compat::*;

use std::marker::PhantomData;
use std::{cmp, fmt, hash, ptr};

// This struct describes the span of a `BitSlice` or `BitSliceMut`, starting
// with of offset of `offset` bits into the array of blocks, and including
// `len` bits.
#[derive(Copy, Clone, Debug)]
struct SliceSpan {
    offset: u8,
    len: u64,
    aligned_blocks: usize,
}
// Invariant:
//   aligned_blocks == if offset == 0 { Block::ceil_div_nbits(len) } else { 0 }

// This struct describes the result of an indexing operation against a span.
// We can give back a full, aligned block, or an arbitrary sequence of bits.
#[derive(Copy, Clone, Debug)]
enum BlockAddress {
    FullBlockAt(usize),
    SomeBitsAt(Address, usize),
}

impl SliceSpan {
    fn new<Block: BlockType>(offset: u8, bit_len: u64) -> Self {
        SliceSpan {
            offset,
            len: bit_len,
            aligned_blocks: if offset == 0 {Block::ceil_div_nbits(bit_len)} else {0},
        }
    }

    fn from_block_len<Block: BlockType>(block_len: usize) -> Self {
        Self::new::<Block>(0, Block::mul_nbits(block_len))
    }

    fn block_len<Block: BlockType>(&self) -> usize {
        Block::ceil_div_nbits(self.len)
    }

    fn find_block<Block: BlockType>(&self, position: usize) -> Option<BlockAddress> {
        if position < self.aligned_blocks {
            return Some(BlockAddress::FullBlockAt(position));
        } else if position < self.block_len::<Block>() {
            let start   = Block::mul_nbits(position) + u64::from(self.offset);
            let address = Address::new::<Block>(start);
            let count   = Block::block_bits(self.len, position);
            Some(BlockAddress::SomeBitsAt(address, count))
        } else {
            None
        }
    }

    fn find_bits<Block: BlockType>(&self, position: u64, count: usize) -> Option<BlockAddress> {
        if position + (count as u64) <= self.len {
            let address = Address::new::<Block>(position + u64::from(self.offset));
            if count == Block::nbits() && address.bit_offset == 0 {
                Some(BlockAddress::FullBlockAt(address.block_index))
            } else {
                Some(BlockAddress::SomeBitsAt(address, count))
            }
        } else {
            None
        }
    }

    fn find_bit<Block: BlockType>(&self, position: u64) -> Option<Address> {
        if position < self.len {
            Some(Address::new::<Block>(position + self.offset as u64))
        } else {
            None
        }
    }
}

impl BlockAddress {
    unsafe fn read<Block: BlockType>(self, bits: *const Block) -> Block {
        match self {
            BlockAddress::FullBlockAt(position) =>
                ptr::read(bits.offset(position as isize)),

            BlockAddress::SomeBitsAt(address, count) => {
                let offset      = address.bit_offset;
                let ptr1        = bits.offset(address.block_index as isize);
                let block1      = ptr::read(ptr1);

                // Otherwise, our access is unaligned and may span two blocks. So we need
                // to get our bits starting at `offset` in `block1`, and the rest from `block2`
                // if necessary.
                let shift1      = offset as usize;
                let shift2      = Block::nbits() - shift1;

                // Getting the right number of bits from `block1`.
                let bits_size1  = cmp::min(shift2, count);
                let chunk1      = block1.get_bits(shift1, bits_size1);

                // The remaining bits will be in `block2`; if there are none, we return early.
                let bits_size2  = count - bits_size1;
                if bits_size2 == 0 {
                    return chunk1;
                }

                // Otherwise, we need to get `block2` and combine bits from each block to get
                // the result.
                let block2      = ptr::read(ptr1.offset(1));
                let chunk2      = block2.get_bits(0, bits_size2);
                (chunk1 | (chunk2 << shift2))
            }
        }
    }

    unsafe fn write<Block: BlockType>(self, bits: *mut Block, value: Block) {
        match self {
            BlockAddress::FullBlockAt(position) =>
                ptr::write(bits.offset(position as isize), value),

            BlockAddress::SomeBitsAt(address, count) => {
                let offset  = address.bit_offset;
                let ptr1    = bits.offset(address.block_index as isize);

                // Otherwise, our access is unaligned. In particular, we need to align
                // the first bits of `value` with the last `Block::nbits() - offset` bits
                // of `block1`, and the last `offset` bits of value with the first bits
                // of `block2`.
                let shift1      = offset;
                let shift2      = Block::nbits() - shift1;

                // We aren't necessarily going to keep all `shift2` bits of `value`, though,
                // because it might exceed the `count`. In any case, we can now read the block,
                // overwrite the correct bits, and write the block back.
                let bits_size1  = cmp::min(count, shift2);
                let old_block1  = ptr::read(ptr1);
                let new_block1  = old_block1.with_bits(shift1, bits_size1, value);
                ptr::write(ptr1, new_block1);

                // The remaining bits to change in `block2`. If it's zero, we finish early.
                let bits_size2  = count - bits_size1;
                if bits_size2 == 0 { return; }
                let ptr2        = ptr1.offset(1);
                let old_block2  = ptr::read(ptr2);
                let new_block2  = old_block2.with_bits(0, bits_size2, value >> shift2);
                ptr::write(ptr2, new_block2);
            }
        }
    }
}

/// A slice of a bit-vector; akin to `&'a [bool]` but packed.
///
/// # Examples
///
/// ```
/// use bv::*;
///
/// let array = [0b00110101u16];
/// let mut slice = array.bit_slice(..8);
/// assert_eq!( slice[0], true );
/// assert_eq!( slice[1], false );
///
/// slice = slice.bit_slice(1..8);
/// assert_eq!( slice[0], false );
/// assert_eq!( slice[1], true );
/// ```
#[derive(Copy, Clone)]
pub struct BitSlice<'a, Block> {
    bits:   *const Block,
    span:   SliceSpan,
    marker: PhantomData<&'a ()>,
}

/// A mutable slice of a bit-vector; akin to `&'a mut [bool]` but packed.
///
/// # Examples
///
/// ```
/// use bv::*;
///
/// let mut array = [0b00110101u16];
///
/// {
///     let mut slice = BitSliceMut::from_slice(&mut array);
///     assert_eq!( slice[0], true );
///     assert_eq!( slice[1], false );
///
///     slice.set_bit(0, false);
/// }
///
/// assert_eq!( array[0], 0b00110100u16 );
/// ```
pub struct BitSliceMut<'a, Block> {
    bits:   *mut Block,
    span:   SliceSpan,
    marker: PhantomData<&'a mut ()>,
}

impl<'a, Block: BlockType> BitSlice<'a, Block> {
    /// Creates a `BitSlice` from an array slice of blocks.
    ///
    /// The size is always a multiple of
    /// `Block::nbits()`. If you want a different size, slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::{BitSlice, BitSliceable};
    ///
    /// let v = vec![0b01010011u16, 0u16];
    /// let slice = BitSlice::from_slice(&v).bit_slice(..7);
    /// assert_eq!( slice.len(), 7 );
    /// assert_eq!( slice[0], true );
    /// assert_eq!( slice[1], true );
    /// assert_eq!( slice[2], false );
    /// ```
    pub fn from_slice(blocks: &'a [Block]) -> Self {
        BitSlice {
            bits:   blocks.as_ptr(),
            span:   SliceSpan::from_block_len::<Block>(blocks.len()),
            marker: PhantomData,
        }
    }

    /// Creates a `BitSlice` from a pointer to its data, an offset where the bits start, and
    /// the number of available bits.
    ///
    /// This is unsafe because the size of the passed-in buffer is
    /// not checked. It must hold at least `offset + len` bits or the resulting behavior is
    /// undefined.
    ///
    /// # Precondition
    ///
    ///   - the first `Block::ceil_div_nbits(len + offset)` words of `bits` safe
    ///     to read.
    pub unsafe fn from_raw_parts(bits: *const Block, offset: u64, len: u64) -> Self {
        let address = Address::new::<Block>(offset);
        BitSlice {
            bits:   bits.offset(address.block_index as isize),
            span:   SliceSpan::new::<Block>(address.bit_offset as u8, len),
            marker: PhantomData,
        }
    }

    /// The number of bits in the slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let bv: BitVec = bit_vec![ true, true, false, true ];
    /// let slice = bv.bit_slice(..3);
    ///
    /// assert_eq!( bv.len(), 4 );
    /// assert_eq!( slice.len(), 3 );
    /// ```
    pub fn len(&self) -> u64 {
        self.span.len
    }

    /// Returns whether there are no bits in the slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let bv: BitVec = bit_vec![ true, true, false, true ];
    /// let slice0 = bv.bit_slice(3..3);
    /// let slice1 = bv.bit_slice(3..4);
    ///
    /// assert!(  slice0.is_empty() );
    /// assert!( !slice1.is_empty() );
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a, Block: BlockType> BitSliceMut<'a, Block> {
    /// Creates a `BitSliceMut` from a mutable array slice of blocks.
    ///
    /// The size is always a multiple of `Block::nbits()`. If you want a different size,
    /// slice.
    pub fn from_slice(blocks: &mut [Block]) -> Self {
        BitSliceMut {
            bits:   blocks.as_mut_ptr(),
            span:   SliceSpan::from_block_len::<Block>(blocks.len()),
            marker: PhantomData,
        }
    }

    /// Creates a `BitSliceMut` from a pointer to its data, an offset where the bits start, and
    /// the number of available bits.
    ///
    /// This is unsafe because the size of the passed-in buffer is
    /// not checked. It must hold at least `offset + len` bits or the resulting behavior is
    /// undefined.
    ///
    /// # Precondition
    ///
    ///   - the first `Block::ceil_div_nbits(len + offset)` words of `bits` safe
    ///     to read and write.
    pub unsafe fn from_raw_parts(bits: *mut Block, offset: u64, len: u64) -> Self {
        let address = Address::new::<Block>(offset);
        BitSliceMut {
            bits:   bits.offset(address.block_index as isize),
            span:   SliceSpan::new::<Block>(address.bit_offset as u8, len),
            marker: PhantomData,
        }
    }

    /// The number of bits in the slice.
    pub fn len(&self) -> u64 {
        self.span.len
    }

    /// Returns whether there are no bits in the slice.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Converts a mutable bit slice to immutable.
    pub fn as_bit_slice(&self) -> BitSlice<'a, Block> {
        BitSlice {
            bits:   self.bits,
            span:   self.span,
            marker: PhantomData,
        }
    }
}

impl<'a, 'b, Block: BlockType> From<&'b BitSliceMut<'a, Block>> for BitSlice<'a, Block> {
    fn from(slice: &'b BitSliceMut<'a, Block>) -> Self {
        slice.as_bit_slice()
    }
}

impl<'a, Block: BlockType> From<&'a [Block]> for BitSlice<'a, Block> {
    fn from(slice: &'a [Block]) -> Self {
        BitSlice::from_slice(slice)
    }
}

impl<'a, Block: BlockType> From<&'a mut [Block]> for BitSliceMut<'a, Block> {
    fn from(slice: &'a mut [Block]) -> Self {
        BitSliceMut::from_slice(slice)
    }
}

// Gets `bits[offset + position]`.
//
// Precondition: Bits are in bounds.
unsafe fn get_raw_bit<Block: BlockType>(bits: *const Block, address: Address) -> bool {

    let ptr       = bits.offset(address.block_index as isize);
    let block     = ptr::read(ptr);
    block.get_bit(address.bit_offset)
}

// Sets `bits[offset + position]`
//
// Precondition: Bit is in bounds.
unsafe fn set_raw_bit<Block: BlockType>(bits: *mut Block, address: Address, value: bool) {
    let ptr       = bits.offset(address.block_index as isize);
    let old_block = ptr::read(ptr);
    let new_block = old_block.with_bit(address.bit_offset, value);
    ptr::write(ptr, new_block);
}

impl<'a, Block: BlockType> Bits for BitSlice<'a, Block> {
    type Block = Block;

    fn bit_len(&self) -> u64 {
        self.len()
    }

    fn get_bit(&self, position: u64) -> bool {
        let address = self.span.find_bit::<Block>(position)
            .expect("BitSlice::get_bit: out of bounds");
        unsafe { get_raw_bit(self.bits, address) }
    }

    fn get_block(&self, position: usize) -> Block {
        get_masked_block(self, position)
    }

    fn get_raw_block(&self, position: usize) -> Block {
        let block_addr = self.span.find_block::<Block>(position)
            .expect("BitSlice::get_block: out of bounds");
        unsafe { block_addr.read(self.bits) }
    }

    fn get_bits(&self, start: u64, count: usize) -> Self::Block {
        let block_addr = self.span.find_bits::<Block>(start, count)
            .expect("BitSlice::get_bits: out of bounds");
        unsafe { block_addr.read(self.bits) }
    }
}

impl<'a, Block: BlockType> Bits for BitSliceMut<'a, Block> {
    type Block = Block;

    fn bit_len(&self) -> u64 {
        self.len()
    }

    fn get_bit(&self, position: u64) -> bool {
        let address = self.span.find_bit::<Block>(position)
            .expect("BitSliceMut::get_bit: out of bounds");
        unsafe { get_raw_bit(self.bits, address) }
    }

    fn get_block(&self, position: usize) -> Block {
        let block_addr = self.span.find_block::<Block>(position)
            .expect("BitSliceMut::get_block: out of bounds");
        unsafe { block_addr.read(self.bits) }
    }

    fn get_bits(&self, start: u64, count: usize) -> Self::Block {
        let block_addr = self.span.find_bits::<Block>(start, count)
            .expect("BitSliceMut::get_bits: out of bounds");
        unsafe { block_addr.read(self.bits) }
    }
}

impl<'a, Block: BlockType> BitsMut for BitSliceMut<'a, Block> {
    fn set_bit(&mut self, position: u64, value: bool) {
        let address = self.span.find_bit::<Block>(position)
            .expect("BitSliceMut::set_bit: out of bounds");
        unsafe {
            set_raw_bit(self.bits, address, value);
        }
    }

    fn set_block(&mut self, position: usize, value: Block) {
        let block_addr = self.span.find_block::<Block>(position)
            .expect("BitSliceMut::set_block: out of bounds");
        unsafe { block_addr.write(self.bits, value); }
    }

    fn set_bits(&mut self, start: u64, count: usize, value: Self::Block) {
        let block_addr = self.span.find_bits::<Block>(start, count)
            .expect("BitSliceMut::set_bits: out of bounds");
        unsafe { block_addr.write(self.bits, value); }
    }
}

impl_index_from_bits! {
    impl['a, Block: BlockType] Index<u64> for BitSlice<'a, Block>;
    impl['a, Block: BlockType] Index<u64> for BitSliceMut<'a, Block>;
}

impl<'a, Block: BlockType> BitSliceable<Range<u64>> for BitSlice<'a, Block> {
    type Slice = Self;

    fn bit_slice(self, range: Range<u64>) -> Self {
        assert!(range.start <= range.end, "BitSlice::slice: bad range");
        assert!(range.end <= self.span.len, "BitSlice::slice: out of bounds");

        unsafe {
            BitSlice::from_raw_parts(self.bits,
                                     range.start + u64::from(self.span.offset),
                                     range.end - range.start)
        }
    }
}

impl<'a, Block: BlockType> BitSliceable<Range<u64>> for BitSliceMut<'a, Block> {
    type Slice = Self;

    fn bit_slice(self, range: Range<u64>) -> Self {
        assert!(range.start <= range.end, "BitSliceMut::slice: bad range");
        assert!(range.end <= self.span.len, "BitSliceMut::slice: out of bounds");

        unsafe {
            BitSliceMut::from_raw_parts(self.bits,
                                        range.start + u64::from(self.span.offset),
                                        range.end - range.start)
        }
    }
}

#[cfg(inclusive_range)]
impl<'a, Block: BlockType> BitSliceable<RangeInclusive<u64>> for BitSlice<'a, Block> {
    type Slice = Self;

    fn bit_slice(self, range: RangeInclusive<u64>) -> Self {
        let (start, end) = get_inclusive_bounds(range)
            .expect("BitSlice::slice: bad range");
        assert!(end < self.span.len, "BitSlice::slice: out of bounds");

        unsafe {
            BitSlice::from_raw_parts(self.bits,
                                     start + u64::from(self.span.offset),
                                     end - start + 1)
        }
    }
}

#[cfg(inclusive_range)]
impl<'a, Block: BlockType> BitSliceable<RangeInclusive<u64>> for BitSliceMut<'a, Block> {
    type Slice = Self;

    fn bit_slice(self, range: RangeInclusive<u64>) -> Self {
        let (start, end) = get_inclusive_bounds(range)
            .expect("BitSliceMut::slice: bad range");
        assert!(end < self.span.len, "BitSliceMut::slice: out of bounds");

        unsafe {
            BitSliceMut::from_raw_parts(self.bits,
                                        start + u64::from(self.span.offset),
                                        end - start + 1)
        }
    }
}

impl<'a, Block: BlockType> BitSliceable<RangeFrom<u64>> for BitSlice<'a, Block> {
    type Slice = Self;

    fn bit_slice(self, range: RangeFrom<u64>) -> Self {
        let len = self.span.len;
        self.bit_slice(range.start .. len)
    }
}

impl<'a, Block: BlockType> BitSliceable<RangeFrom<u64>> for BitSliceMut<'a, Block> {
    type Slice = Self;

    fn bit_slice(self, range: RangeFrom<u64>) -> Self {
        let len = self.span.len;
        self.bit_slice(range.start .. len)
    }
}

impl<'a, Block: BlockType> BitSliceable<RangeTo<u64>> for BitSlice<'a, Block> {
    type Slice = Self;

    fn bit_slice(self, range: RangeTo<u64>) -> Self {
        self.bit_slice(0 .. range.end)
    }
}

impl<'a, Block: BlockType> BitSliceable<RangeTo<u64>> for BitSliceMut<'a, Block> {
    type Slice = Self;

    fn bit_slice(self, range: RangeTo<u64>) -> Self {
        self.bit_slice(0 .. range.end)
    }
}

#[cfg(inclusive_range)]
impl<'a, Block: BlockType> BitSliceable<RangeToInclusive<u64>> for BitSlice<'a, Block> {
    type Slice = Self;

    fn bit_slice(self, range: RangeToInclusive<u64>) -> Self {
        self.bit_slice(0 .. range.end + 1)
    }
}

#[cfg(inclusive_range)]
impl<'a, Block: BlockType> BitSliceable<RangeToInclusive<u64>> for BitSliceMut<'a, Block> {
    type Slice = Self;

    fn bit_slice(self, range: RangeToInclusive<u64>) -> Self {
        self.bit_slice(0 .. range.end + 1)
    }
}

impl<'a, Block: BlockType> BitSliceable<RangeFull> for BitSlice<'a, Block> {
    type Slice = Self;

    fn bit_slice(self, _: RangeFull) -> Self {
        self
    }
}

impl<'a, Block: BlockType> BitSliceable<RangeFull> for BitSliceMut<'a, Block> {
    type Slice = Self;

    fn bit_slice(self, _: RangeFull) -> Self {
        self
    }
}

impl<'a, Block, R> BitSliceable<R> for &'a [Block]
    where Block: BlockType,
          BitSlice<'a, Block>: BitSliceable<R, Block = Block, Slice = BitSlice<'a, Block>> {

    type Slice = BitSlice<'a, Block>;

    fn bit_slice(self, range: R) -> Self::Slice {
        BitSlice::from_slice(self).bit_slice(range)
    }
}

impl<'a, Block, R> BitSliceable<R> for &'a mut [Block]
    where Block: BlockType,
          BitSliceMut<'a, Block>: BitSliceable<R, Block = Block, Slice = BitSliceMut<'a, Block>> {

    type Slice = BitSliceMut<'a, Block>;

    fn bit_slice(self, range: R) -> Self::Slice {
        BitSliceMut::from_slice(self).bit_slice(range)
    }
}

impl<'a, Other: Bits> PartialEq<Other> for BitSlice<'a, Other::Block> {
    fn eq(&self, other: &Other) -> bool {
        BlockIter::new(self) == BlockIter::new(other)
    }
}

impl<'a, Block: BlockType> Eq for BitSlice<'a, Block> {}

impl<'a, Block: BlockType> PartialOrd for BitSlice<'a, Block> {
    fn partial_cmp(&self, other: &BitSlice<Block>) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a, Block: BlockType> Ord for BitSlice<'a, Block> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        let iter1 = BlockIter::new(*self);
        let iter2 = BlockIter::new(*other);
        (iter1).cmp(iter2)
    }
}

impl<'a, Other: Bits> PartialEq<Other> for BitSliceMut<'a, Other::Block> {
    fn eq(&self, other: &Other) -> bool {
        BlockIter::new(self) == BlockIter::new(other)
    }
}

impl<'a, Block: BlockType> Eq for BitSliceMut<'a, Block> {}

impl<'a, Block: BlockType> PartialOrd for BitSliceMut<'a, Block> {
    fn partial_cmp(&self, other: &BitSliceMut<Block>) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a, Block: BlockType> Ord for BitSliceMut<'a, Block> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_bit_slice().cmp(&other.as_bit_slice())
    }
}

impl<'a, Block: BlockType + hash::Hash> hash::Hash for BitSlice<'a, Block> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.bit_len());
        for block in BlockIter::new(self) {
            block.hash(state);
        }
    }
}

impl<'a, Block: BlockType + hash::Hash> hash::Hash for BitSliceMut<'a, Block> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_bit_slice().hash(state);
    }
}

impl<'a, Block: BlockType> fmt::Debug for BitSlice<'a, Block> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "bit_vec![")?;
        if !self.is_empty() {
            write!(f, "{}", self.get_bit(0))?;
        }
        for i in 1 .. self.span.len {
            write!(f, ", {}", self.get_bit(i))?;
        }
        write!(f, "]")
    }
}

impl<'a, Block: BlockType> fmt::Debug for BitSliceMut<'a, Block> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_bit_slice().fmt(f)
    }
}

#[cfg(test)]
mod test {
    use BitVec;
    use super::*;

    #[test]
    fn bit_slice_from_slice() {
        let mut bytes = [0b00001111u8];
        {
            let mut bs = BitSliceMut::from_slice(&mut bytes);
            assert_eq!( bs.get_block(0), 0b00001111 );
            bs.set_bit(1, false);
            assert_eq!( bs.get_block(0), 0b00001101 );
        }

        assert_eq!( bytes[0], 0b00001101 );
    }

    #[test]
    fn bit_slice_index() {
        let mut bytes = [0b00001111u8];
        {
            let bs = BitSlice::from_slice(&bytes);
            assert_eq!( bs[3], true );
            assert_eq!( bs[4], false );
        }
        {
            let bs = BitSliceMut::from_slice(&mut bytes);
            assert_eq!( bs[3], true );
            assert_eq!( bs[4], false );
        }
    }

    #[test]
    fn bit_slice_update_across_blocks() {
        let mut bv: BitVec<u8> = bit_vec![ true; 20 ];
        bv.set_bit(3, false);
        bv.set_bit(7, false);

        {
            let mut slice: BitSliceMut<u8> = (&mut bv).bit_slice(4..12);
            slice.set_bit(1, false);
            slice.set_bit(5, false);
        }

        assert!(  bv[0] );
        assert!(  bv[1] );
        assert!(  bv[2] );
        assert!( !bv[3] );
        assert!(  bv[4] );
        assert!( !bv[5] );
        assert!(  bv[6] );
        assert!( !bv[7] );
        assert!(  bv[8] );
        assert!( !bv[9] );
    }

    #[test]
    fn debug_for_bit_slice() {
        let slice = [0b00110101u8];
        let bs = BitSlice::from_slice(&slice);
        let exp = "bit_vec![true, false, true, false, true, true, false, false]";
        let act = format!("{:?}", bs);
        assert_eq!( act, exp );
    }

    #[cfg(inclusive_range)]
    #[test]
    fn range_to_inclusive() {
        use BitSliceable;

        let base = [0b00110101u8];
        let slice = base.bit_slice(::std::ops::RangeToInclusive { end: 4 });
        assert_eq!( slice.len(), 5 );
    }
}

