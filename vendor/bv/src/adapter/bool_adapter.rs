use {Bits, BitsMut, BitsPush};
use BlockType;
use iter::BlockIter;

use std::marker::PhantomData;
use std::ops;

/// Adapts a sequence of `bool`s (*e.g.,* `&[bool]`) to emulate a bit
/// vector.
///
/// In particular, this adapter implements [`Bits`], [`BitsMut`], and
/// [`BitsPush`] as appropriate. It implement `PartialEq<T>` for all
/// `T: Bits<Block=Block>`. It does not, however, implement slicing, so
/// slice before you adapt.
///
/// Note that a bare `Vec<bool>` or `&[bool]` already implements [`Bits`],
/// etc., with a `Block` type of `u8`. This means that it is only
/// compatible with other `u8`-based bit vectors. `BoolAdapter` is instead
/// parametrized by the block type, so it works with bit vectors, slices,
/// and adapters of any uniform block type.
///
/// [`Bits`]: ../trait.Bits.html
/// [`BitsMut`]: ../trait.BitsMut.html
/// [`BitsPush`]: ../trait.BitsPush.html
#[derive(Debug, Clone)]
pub struct BoolAdapter<Block, T> {
    bits:    T,
    _marker: PhantomData<Block>,
}

impl<Block: BlockType, T> BoolAdapter<Block, T> {
    /// Creates a new `BoolAdapter` from an underlying sequence of `bool`s.
    ///
    /// Note that the `BoolAdapter` derefs to the underlying `bool` sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use bv::BitSliceable;
    /// use bv::adapter::BoolAdapter;
    ///
    /// let array = [0b101usize];
    /// let bv1 = BoolAdapter::new(vec![true, false, true]);
    /// let bv2 = array.bit_slice(0..3);
    /// assert_eq!( bv1, bv2 );
    /// ```
    pub fn new(bits: T) -> Self {
        BoolAdapter {
            bits,
            _marker: PhantomData,
        }
    }

    /// Gets the underlying `bool` sequence object back out of a `BoolAdapter`.
    pub fn into_inner(self) -> T {
        self.bits
    }
}

impl<Block, T> ops::Deref for BoolAdapter<Block, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.bits
    }
}

impl<Block, T> ops::DerefMut for BoolAdapter<Block, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.bits
    }
}

macro_rules! impl_for_bool_adapter {
    () => {};

    (
        impl[$($param:tt)*] Bits for BoolAdapter<$block:ty, $target:ty>;
        $( $rest:tt )*
    ) => {
        impl<$($param)*> Bits for BoolAdapter<$block, $target> {
            type Block = $block;

            fn bit_len(&self) -> u64 {
                self.bits.len() as u64
            }

            fn get_bit(&self, position: u64) -> bool {
                self.bits[position as usize]
            }
        }

        impl_for_bool_adapter! { $($rest)* }
    };

    (
        impl[$($param:tt)*] BitsMut for BoolAdapter<$block:ty, $target:ty>;
        $( $rest:tt )*
    ) => {
        impl<$($param)*> BitsMut for BoolAdapter<$block, $target> {
            fn set_bit(&mut self, position: u64, value: bool) {
                self.bits[position as usize] = value
            }
        }

        impl_for_bool_adapter! { $($rest)* }
    };

    (
        impl[$($param:tt)*] BitsPush for BoolAdapter<$block:ty, $target:ty>;
        $( $rest:tt )*
    ) => {
        impl<$($param)*> BitsPush for BoolAdapter<$block, $target> {
            fn push_bit(&mut self, value: bool) {
                self.bits.push(value);
            }

            fn pop_bit(&mut self) -> Option<bool> {
                self.bits.pop()
            }
        }

        impl_for_bool_adapter! { $($rest)* }
    };
}

impl_for_bool_adapter! {
    impl[    Block: BlockType] Bits     for BoolAdapter<Block, Vec<bool>>;
    impl[    Block: BlockType] BitsMut  for BoolAdapter<Block, Vec<bool>>;
    impl[    Block: BlockType] BitsPush for BoolAdapter<Block, Vec<bool>>;

    impl['a, Block: BlockType] Bits     for BoolAdapter<Block, &'a mut Vec<bool>>;
    impl['a, Block: BlockType] BitsMut  for BoolAdapter<Block, &'a mut Vec<bool>>;
    impl['a, Block: BlockType] BitsPush for BoolAdapter<Block, &'a mut Vec<bool>>;

    impl['a, Block: BlockType] Bits     for BoolAdapter<Block, &'a mut [bool]>;
    impl['a, Block: BlockType] BitsMut  for BoolAdapter<Block, &'a mut [bool]>;

    impl['a, Block: BlockType] Bits     for BoolAdapter<Block, &'a [bool]>;
}

impl<Block, T, U> PartialEq<U> for BoolAdapter<Block, T>
    where Block: BlockType,
          U: Bits<Block = Block>,
          Self: Bits<Block = Block> {

    fn eq(&self, other: &U) -> bool {
        BlockIter::new(self) == BlockIter::new(other)
    }
}

