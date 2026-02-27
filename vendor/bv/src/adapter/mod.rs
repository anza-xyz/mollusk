//! Lazy bit vector adapters.
//!
//! This module defines adapters for dealing with bit vectors and other
//! types that implement [`Bits`]. It also defines the adaptors that
//! are returned by methods of the extension trait [`BitsExt`].
//!
//! [`Bits`]: ../trait.Bits.html
//! [`BitsExt`]: ../trait.BitsExt.html

#[macro_use]
mod bit_slice_adapter;
pub use self::bit_slice_adapter::BitSliceAdapter;

mod logic;
pub use self::logic::{BitNot, BitAnd, BitOr, BitXor, BitZip};

mod bit_fill;
pub use self::bit_fill::BitFill;

mod bit_concat;
pub use self::bit_concat::BitConcat;

mod bool_adapter;
pub use self::bool_adapter::BoolAdapter;

#[cfg(test)]
mod test {
    use {Bits, BitsExt, BitsMut, BitVec, BitSliceable};
    use super::BitSliceAdapter;

    fn assert_0001<T: Bits>(bits: &T) {
        assert_eq!( bits.bit_len(), 4 );

        assert!( !bits.get_bit(0) );
        assert!( !bits.get_bit(1) );
        assert!( !bits.get_bit(2) );
        assert!(  bits.get_bit(3) );

        let bv = bits.to_bit_vec();
        assert_eq!( bv, bit_vec![false, false, false, true] );
    }

    #[test]
    fn simple_not() {
        let bv: BitVec = bit_vec![true, true, true, false,];
        let not_bits = bv.bit_not();
        assert_0001(&not_bits);
    }

    #[test]
    fn simple_and() {
        let bv1: BitVec<u8> = bit_vec![ false, false, true, true, ];
        let bv2: BitVec<u8> = bit_vec![ false, true, false, true, ];
        let and_bits = bv1.bit_and(&bv2);
        assert_0001(&and_bits);
    }

    #[test]
    fn and_with_same_offset() {
        let bv1: BitVec<u8> = bit_vec![ true, false, false, true, true ];
        let bv2: BitVec<u8> = bit_vec![ true, false, true, false, true ];
        let bv_slice1 = bv1.bit_slice(1..);
        let bv_slice2 = bv2.bit_slice(1..);
        let and_bits = bv_slice1.bit_and(&bv_slice2);
        assert_0001(&and_bits);
    }

    #[test]
    fn and_with_different_offset() {
        let bv1: BitVec<u8> = bit_vec![ true, true, false, false, true, true ];
        let bv2: BitVec<u8> = bit_vec![ true, false, true, false, true ];
        let bv_slice1 = bv1.bit_slice(2..);
        let bv_slice2 = bv2.bit_slice(1..);
        let and_bits = bv_slice1.bit_and(&bv_slice2);
        assert_0001(&and_bits);
    }

    #[test]
    fn append() {
        let bv1: BitVec<u8> = bit_vec![false];
        let bv2: BitVec<u8> = bit_vec![true, true];
        let bv3: BitVec<u8> = bit_vec![false, false, false];

        let bv123 = bv1.bit_concat(&bv2).into_bit_concat(&bv3);
        let app12 = bv123.bit_concat(&bv123);
        let app24 = app12.bit_concat(&app12);
        let bv = BitVec::from_bits(&app24);

        assert_eq!(bv, bit_vec![false, true, true, false, false, false,
                                false, true, true, false, false, false,
                                false, true, true, false, false, false,
                                false, true, true, false, false, false]);
    }

    #[test]
    fn pad() {
        let bv1: BitVec = bit_vec![true, false, true, false];
        let bv2: BitVec = bit_vec![true, true];

        let bv3 = bv1.bit_or(bv2.bit_pad(bv1.bit_len())).to_bit_vec();

        assert_eq!( bv3, bit_vec![true, true, true, false] );
    }

    #[test]
    fn slice_adapter() {
        let bv1: BitVec = bit_vec![true, false, true, false, true, false, true, false];
        let bv2: BitVec = bit_vec![true, true, false, false, true, true, false, false];

        let bv3 = bv1.bit_xor(&bv2).bit_slice(1..7).to_bit_vec();

        assert_eq!( bv3, bit_vec![true, true, false, false, true, true] );
    }

    #[test]
    fn slice_adapter_mutation() {
        let mut bv: BitVec = bit_vec![true, false, true, false];

        {
            let mut slice = BitSliceAdapter::new(&mut bv, 1, 2);
            slice.set_bit(1, false);
        }

        assert_eq!( bv, bit_vec![true, false, false, false] );

        {
            let mut slice = BitSliceAdapter::new(&mut bv, 1, 2);
            slice.set_block(0, 0b111);
        }

        assert_eq!( bv, bit_vec![true, true, true, false] );
    }

    #[test]
    fn mixed_equality() {
        let bv1: BitVec = bit_vec![false, false, true, true];
        let bv2: BitVec = bit_vec![false, true, false, true];
        assert_eq!( bv1.bit_xor(&bv2), bit_vec![false, true, true, false] );
    }
}
