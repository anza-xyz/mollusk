use super::storage::*;
use super::slice::*;
use super::traits::*;

use std::cmp::{max, Ordering};
use std::ptr;

mod inner;
use self::inner::Inner;

mod impls;

#[cfg(test)]
mod test;

/// A bit-vector, akin to `Vec<bool>` but packed.
///
/// `BitVec` stores its bits in an array of `Block`s, where `Block` is given as a type parameter
/// that defaults to `usize`. You might find that a different `Block` size is preferable, but
/// only benchmarking will tell.
///
/// Several useful methods are exported in traits, rather than inherent to `BitVec`. In
/// particular, see:
///
///   - [`Bits::get_bit`](trait.Bits.html#method.get_bit) and
///   - [`BitsMut::set_bit`](trait.BitsMut.html#method.set_bit).
///
/// You will likely want to `use` these traits (or `bv::*`) when you use `BitVec`.
///
/// # Examples
///
/// ```
/// use bv::BitVec;
///
/// let mut bv: BitVec = BitVec::new();
/// assert_eq!(bv.len(), 0);
///
/// bv.push(true);
/// bv.push(false);
/// bv.push(true);
/// assert_eq!(bv.len(), 3);
///
/// assert_eq!(bv[0], true);
/// assert_eq!(bv[1], false);
/// assert_eq!(bv[2], true);
/// ```
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct BitVec<Block: BlockType = usize> {
    bits: Inner<Block>,
    len: u64,
}
// Invariant: self.invariant()

#[cfg(feature = "serde")]
impl<'de, Block: BlockType + serde::Deserialize<'de>> serde::Deserialize<'de> for BitVec<Block> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Unchecked<Block: BlockType> {
            bits: Inner<Block>,
            len: u64,
        }
        let unchecked = Unchecked::deserialize(deserializer)?;
        let unchecked = BitVec {
            bits: unchecked.bits,
            len: unchecked.len,
        };
        if !unchecked.invariant() {
            return Err(serde::de::Error::custom("Invalid BitVec"));
        }
        Ok(unchecked)
    }
}

impl<Block: BlockType> Default for BitVec<Block> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Block: BlockType> BitVec<Block> {
    #[allow(dead_code)]
    fn invariant(&self) -> bool {
        return self.len <= Block::mul_nbits(self.bits.len());
    }

    /// Creates a new, empty bit-vector with a capacity of one block.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::BitVec;
    ///
    /// let mut bv: BitVec = BitVec::new();
    /// assert_eq!(bv.len(), 0);
    ///
    /// bv.push(true);
    /// bv.push(false);
    /// bv.push(true);
    /// assert_eq!(bv.len(), 3);
    ///
    /// assert_eq!(bv[0], true);
    /// assert_eq!(bv[1], false);
    /// assert_eq!(bv[2], true);
    /// ```
    pub fn new() -> Self {
        Self::with_block_capacity(0)
    }

    /// Creates a new, empty bit-vector with the given bit capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::BitVec;
    ///
    /// let mut bv: BitVec<u16> = BitVec::with_capacity(20);
    /// assert_eq!(bv.capacity(), 32);
    /// ```
    pub fn with_capacity(nbits: u64) -> Self {
        Self::with_block_capacity(Block::ceil_div_nbits(nbits))
    }

    /// Creates a new, empty bit-vector with the given block capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::BitVec;
    ///
    /// let mut bv: BitVec<u16> = BitVec::with_block_capacity(8);
    /// assert_eq!(bv.capacity(), 128);
    /// ```
    pub fn with_block_capacity(nblocks: usize) -> Self {
        let mut result = Self::from_block(Block::zero(), nblocks);
        result.len = 0;
        result
    }

    /// Creates a new bit-vector of size `len`, filled with all 0s or 1s
    /// depending on `value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let mut bv: BitVec<u64> = BitVec::new_fill(false, 100);
    ///
    /// assert_eq!( bv.get(0), false );
    /// assert_eq!( bv.len(), 100 );
    /// ```
    pub fn new_fill(value: bool, len: u64) -> Self {
        let mut result = Self::new_block_fill(value, Block::ceil_div_nbits(len));
        result.len = len;
        result
    }

    /// Creates a new bit-vector filled with `value`, made up of `nblocks` blocks.
    #[inline]
    fn new_block_fill(value: bool, nblocks: usize) -> Self {
        let block = if value {!Block::zero()} else {Block::zero()};
        Self::from_block(block, nblocks)
    }

    #[inline]
    fn from_block(init: Block, nblocks: usize) -> Self {
        BitVec {
            bits: Inner::new(init, nblocks),
            len:  Block::mul_nbits(nblocks),
        }
    }

    // Reallocates to have the given capacity.
    fn reallocate(&mut self, block_cap: usize) {
        // We know this is safe because the precondition on `close_resize` is
        // that the first argument gives a valid number of blocks to copy.
        unsafe {
            self.bits = self.bits.clone_resize(self.block_len(), block_cap);
        }
    }

    /// Creates a new `BitVec` from any value implementing the `Bits` trait with
    /// the same block type.
    pub fn from_bits<B: Bits<Block = Block>>(bits: B) -> Self {
        let block_len = bits.block_len();

        let mut vec: Vec<Block> = Vec::with_capacity(block_len);

        // This is safe because we just allocated the vector to the right size,
        // and we fully initialize the vector before setting the size.
        unsafe {
            let ptr = vec.as_mut_ptr();

            for i in 0 .. block_len {
                ptr::write(ptr.offset(i as isize), bits.get_raw_block(i));
            }

            vec.set_len(block_len);
        }

        let mut result: BitVec<Block> = vec.into();
        result.resize(bits.bit_len(), false);
        result
    }

    /// The number of bits in the bit-vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::BitVec;
    ///
    /// let mut bv: BitVec = BitVec::new();
    /// assert_eq!(bv.len(), 0);
    /// bv.push(false);
    /// assert_eq!(bv.len(), 1);
    /// bv.push(false);
    /// assert_eq!(bv.len(), 2);
    /// bv.push(false);
    /// assert_eq!(bv.len(), 3);
    /// ```
    #[inline]
    pub fn len(&self) -> u64 {
        self.len
    }

    /// The number of blocks used by this bit-vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let mut bv: BitVec<u64> = BitVec::new_fill(false, 100);
    ///
    /// assert_eq!( bv.len(), 100 );
    /// assert_eq!( bv.block_len(), 2 );
    /// ```
    pub fn block_len(&self) -> usize {
        Block::ceil_div_nbits(self.len())
    }

    /// The capacity of the bit-vector in bits.
    ///
    /// This is the number of bits that can be held without reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let bv: BitVec<u64> = bit_vec![false; 100];
    ///
    /// assert_eq!( bv.len(), 100 );
    /// assert_eq!( bv.capacity(), 128 );
    /// ```
    ///
    /// Note that this example holds because `bit_vec!` does not introduces excess
    /// capacity.
    pub fn capacity(&self) -> u64 {
        Block::mul_nbits(self.block_capacity())
    }

    /// The capacity of the bit-vector in blocks.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let bv: BitVec<u64> = BitVec::with_capacity(250);
    ///
    /// assert_eq!( bv.len(), 0 );
    /// assert_eq!( bv.block_len(), 0 );
    /// assert_eq!( bv.capacity(), 256 );
    /// assert_eq!( bv.block_capacity(), 4 );
    /// ```
    ///
    /// Note that this example holds because `bit_vec!` does not introduces excess
    /// capacity.
    pub fn block_capacity(&self) -> usize {
        self.bits.len()
    }

    /// Adjust the capacity to hold at least `additional` additional bits.
    ///
    /// May reserve more to avoid frequent reallocations.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let mut bv: BitVec<u32> = bit_vec![ false, false, true ];
    /// assert_eq!( bv.capacity(), 32 );
    /// bv.reserve(100);
    /// assert!( bv.capacity() >= 103 );
    /// ```
    pub fn reserve(&mut self, additional: u64) {
        let old_cap = self.capacity();
        let req_cap = self.len() + additional;
        if req_cap > old_cap {
            self.reserve_exact(max(additional, old_cap));
        }
    }

    /// Adjust the capacity to hold at least `additional` additional blocks.
    ///
    /// May reserve more to avoid frequent reallocations.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let mut bv: BitVec<u32> = bit_vec![ false, false, true ];
    /// assert_eq!( bv.block_capacity(), 1 );
    /// bv.block_reserve(3);
    /// assert!( bv.block_capacity() >= 4 );
    /// ```
    pub fn block_reserve(&mut self, additional: usize) {
        let old_cap = self.block_capacity();
        let req_cap = self.block_len() + additional;
        if req_cap > old_cap {
            self.block_reserve_exact(max(additional, old_cap));
        }
    }

    /// Adjust the capacity to hold at least `additional` additional bits.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let mut bv: BitVec<u32> = bit_vec![ false, false, true ];
    /// assert_eq!( bv.capacity(), 32 );
    /// bv.reserve_exact(100);
    /// assert_eq!( bv.capacity(), 128 );
    /// ```
    pub fn reserve_exact(&mut self, additional: u64) {
        let new_cap = Block::ceil_div_nbits(self.len() + additional);
        if new_cap > self.block_capacity() {
            self.reallocate(new_cap);
        }
    }

    /// Adjusts the capacity to at least `additional` blocks beyond those used.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let mut bv: BitVec<u32> = bit_vec![ false, false, true ];
    /// assert_eq!( bv.block_capacity(), 1 );
    /// bv.block_reserve_exact(3);
    /// assert_eq!( bv.block_capacity(), 4 );
    /// ```
    pub fn block_reserve_exact(&mut self, additional: usize) {
        let new_cap = self.block_len() + additional;
        if new_cap > self.block_capacity() {
            self.reallocate(new_cap);
        }
    }

    /// Shrinks the capacity of the vector as much as possible.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::BitVec;
    ///
    /// let mut bv: BitVec<u8> = BitVec::new();
    ///
    /// for i in 0 .. 23 {
    ///     bv.push(i % 3 == 0);
    /// }
    ///
    /// assert!(bv.capacity() >= 24);
    ///
    /// bv.shrink_to_fit();
    /// assert_eq!(bv.capacity(), 24);
    /// ```
    pub fn shrink_to_fit(&mut self) {
        let block_len = self.block_len();
        if self.block_capacity() > block_len {
            self.reallocate(block_len);
        }
    }

    /// Converts the vector into `Box<[Block]>`.
    ///
    /// Note that this will *not* drop any excess capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let bv: BitVec<u8> = bit_vec![true, true, false, false, true, false, true, false];
    /// let bs = bv.into_boxed_slice();
    ///
    /// assert!( bs.len() >= 1 );
    /// assert_eq!( bs[0], 0b01010011 );
    /// ```
    pub fn into_boxed_slice(self) -> Box<[Block]> {
        self.bits.into_boxed_slice()
    }

    /// Shortens the vector, keeping the first `len` elements and dropping the rest.
    ///
    /// If `len` is greater than the vector's current length, this has no effect.
    ///
    /// Note that this method has no effect on the capacity of the bit-vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let mut v1: BitVec = bit_vec![ true, true, false, false ];
    /// let     v2: BitVec = bit_vec![ true, true ];
    ///
    /// assert_ne!( v1, v2 );
    ///
    /// v1.truncate(2);
    ///
    /// assert_eq!( v1, v2 );
    /// ```
    pub fn truncate(&mut self, len: u64) {
        if len < self.len {
            self.len = len;
        }
    }

    /// Resizes the bit-vector, filling with `value` if it has to grow.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let     v1: BitVec = bit_vec![ true, true, false, false ];
    /// let mut v2: BitVec = bit_vec![ true, true ];
    /// let mut v3: BitVec = bit_vec![ true, true ];
    ///
    /// v2.resize(4, false);
    /// v3.resize(4, true);
    ///
    /// assert_eq!( v1, v2 );
    /// assert_ne!( v1, v3 );
    /// ```
    pub fn resize(&mut self, len: u64, value: bool) {
        match len.cmp(&self.len) {
            Ordering::Less => {
                self.len = len
            },
            Ordering::Equal => { },
            Ordering::Greater => {
                {
                    let growth = len - self.len();
                    self.reserve(growth);
                }

                self.align_block(value);

                let block = if value {!Block::zero()} else {Block::zero()};
                while self.len < len {
                    self.push_block(block);
                }

                self.len = len;
            },
        }
    }

    /// Gets a slice to a `BitVec`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let bv: BitVec = bit_vec![true, false, true];
    /// let slice = bv.as_slice();
    ///
    /// assert_eq!( slice.len(), 3 );
    /// assert_eq!( slice[0], true );
    /// assert_eq!( slice[1], false );
    /// assert_eq!( slice[2], true );
    /// ```
    pub fn as_slice(&self) -> BitSlice<Block> {
        // We know this is safe because the precondition on `from_raw_parts` is
        // that all the bits be in bounds. If `self.len == 0` then there are no
        // bits to access, so it's okay that the pointer dangles. Otherwise, the
        // bits from `0 .. self.len` are in bounds.
        unsafe {
            BitSlice::from_raw_parts(self.bits.as_ptr(), 0, self.len)
        }
    }

    /// Gets a mutable slice to a `BitVec`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let mut bv: BitVec = bit_vec![true, false, true];
    ///
    /// {
    ///     let mut slice = bv.as_mut_slice();
    ///     slice.set_bit(1, true);
    /// }
    ///
    /// assert_eq!( bv[1], true );
    /// ```
    pub fn as_mut_slice(&mut self) -> BitSliceMut<Block> {
        // We know this is safe for the same reason that `as_slice` is safe.
        unsafe {
            BitSliceMut::from_raw_parts(self.bits.as_mut_ptr(), 0, self.len)
        }
    }

    /// Gets the value of the bit at the given position.
    ///
    /// This is an alias for [`Bits::get_bit`].
    ///
    /// # Panics
    ///
    /// If the position is out of bounds.
    ///
    /// [`Bits::get_bit`]: ../trait.Bits.html#get_bit.method
    pub fn get(&self, position: u64) -> bool {
        self.get_bit(position)
    }

    /// Sets the value of the bit at the given position.
    ///
    /// This is an alias for [`BitsMut::set_bit`].
    ///
    /// # Panics
    ///
    /// If the position is out of bounds.
    ///
    /// [`BitsMut::set_bit`]: ../trait.BitsMut.html#set_bit.method
    pub fn set(&mut self, position: u64, value: bool) {
        self.set_bit(position, value);
    }

    /// Adds the given `bool` to the end of the bit-vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let mut bv0: BitVec = bit_vec![ ];
    /// let     bv1: BitVec = bit_vec![ true ];
    /// let     bv2: BitVec = bit_vec![ true, false ];
    /// let     bv3: BitVec = bit_vec![ true, false, true ];
    ///
    /// assert_ne!( bv0, bv1 );
    /// assert_ne!( bv0, bv2 );
    /// assert_ne!( bv0, bv3 );
    ///
    /// bv0.push(true);
    /// assert_eq!( bv0, bv1 );
    ///
    /// bv0.push(false);
    /// assert_eq!( bv0, bv2 );
    ///
    /// bv0.push(true);
    /// assert_eq!( bv0, bv3 );
    /// ```
    pub fn push(&mut self, value: bool) {
        self.reserve(1);
        let old_len = self.len;
        self.len = old_len + 1;
        self.set_bit(old_len, value);
    }

    /// Removes and returns the last element of the bit-vector, or `None` if empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let mut bv: BitVec = bit_vec![ true, false, true ];
    /// assert_eq!( bv.pop(), Some(true) );
    /// assert_eq!( bv.pop(), Some(false) );
    /// assert_eq!( bv.pop(), Some(true) );
    /// assert_eq!( bv.pop(), None );
    /// ```
    pub fn pop(&mut self) -> Option<bool> {
        if self.len > 0 {
            let new_len = self.len - 1;
            let result = self.get_bit(new_len);
            self.len = new_len;
            Some(result)
        } else {
            None
        }
    }

    /// Removes all elements from the bit-vector.
    ///
    /// Does not change the capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let mut bv: BitVec<u32> = bit_vec![ true ];
    /// assert_eq!( bv.len(), 1 );
    /// assert_eq!( bv.capacity(), 32 );
    /// bv.clear();
    /// assert_eq!( bv.len(), 0 );
    /// assert_eq!( bv.capacity(), 32 );
    /// ```
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Does the bit-vector have no elements?
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::*;
    ///
    /// let mut bv: BitVec<u32> = bit_vec![ true ];
    /// assert!( !bv.is_empty() );
    /// bv.clear();
    /// assert!(  bv.is_empty() );
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}
