mod bits;
pub use self::bits::Bits;
pub (crate) use self::bits::get_masked_block;

mod bits_ext;
pub use self::bits_ext::BitsExt;

mod bits_mut;
pub use self::bits_mut::BitsMut;

mod bits_mut_ext;
pub use self::bits_mut_ext::BitsMutExt;

mod bits_push;
pub use self::bits_push::BitsPush;

mod bit_sliceable;
pub use self::bit_sliceable::{BitSliceable, BitSliceableMut};

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn vec_u8_is_bit_vec() {
        let v = vec![0b01001000u8, 0b11100011u8];
        assert!( !v.get_bit(0) );
        assert!( !v.get_bit(1) );
        assert!( !v.get_bit(2) );
        assert!(  v.get_bit(3) );
        assert!( !v.get_bit(4) );
        assert!( !v.get_bit(5) );
        assert!(  v.get_bit(6) );
        assert!( !v.get_bit(7) );
        assert!(  v.get_bit(8) );
        assert!(  v.get_bit(9) );
        assert!( !v.get_bit(10) );
        assert!( !v.get_bit(11) );
        assert!( !v.get_bit(12) );
        assert!(  v.get_bit(13) );
        assert!(  v.get_bit(14) );
        assert!(  v.get_bit(15) );

        assert_eq!( v.get_bits(4, 8), 0b00110100u8 );
    }

    #[test]
    fn vec_u8_is_bit_vec_mut() {
        let mut v = vec![0b01001000u8, 0b11100011u8];
        assert!( !v.get_bit(0) );
        v.set_bit(0, true);
        assert!(  v.get_bit(0) );
        assert!( !v.get_bit(1) );
        v.set_bit(1, true);
        assert!(  v.get_bit(1) );
        assert!( !v.get_bit(10) );
        v.set_bit(10, true);
        assert!(  v.get_bit(10) );

        v.set_bits(4, 8, 0b11110000);

        assert!( !v.get_bit(4) );
        assert!( !v.get_bit(5) );
        assert!( !v.get_bit(6) );
        assert!( !v.get_bit(7) );
        assert!(  v.get_bit(8) );
        assert!(  v.get_bit(9) );
        assert!(  v.get_bit(10) );
        assert!(  v.get_bit(11) );
    }

    #[test]
    fn bogus_get_bits_vec_bool_works_okay() {
        let v = vec![ true, false, false, true, false, true, true, false,
                      false, true, true, false, true, false, false, true ];

        assert_eq!( v.bit_len(), 16 );
        assert_eq!( v.block_len(), 2 );

        assert!(  v.get_bit(0) );
        assert!( !v.get_bit(1) );
        assert!( !v.get_bit(2) );
        assert!(  v.get_bit(3) );

        assert_eq!( v.get_bits(0, 8), 0b01101001 );
        assert_eq!( v.get_bits(0, 7), 0b01101001 );
        assert_eq!( v.get_bits(0, 6), 0b00101001 );

        assert_eq!( v.get_bits(3, 5), 0b00001101 );
        assert_eq!( v.get_bits(3, 6), 0b00001101 );
        assert_eq!( v.get_bits(3, 7), 0b01001101 );
        assert_eq!( v.get_bits(3, 8), 0b11001101 );
        assert_eq!( v.get_bits(4, 8), 0b01100110 );
        assert_eq!( v.get_bits(5, 8), 0b10110011 );
        assert_eq!( v.get_bits(6, 8), 0b01011001 );
        assert_eq!( v.get_bits(7, 8), 0b00101100 );
        assert_eq!( v.get_bits(8, 8), 0b10010110 );
    }

    #[test]
    #[should_panic]
    fn bogus_get_bits_vec_bool_oob() {
        let v = vec![ false; 16 ];
        v.get_bits(9, 8);
    }

    #[test]
    #[should_panic]
    fn get_block_oob() {
        let v = vec![ false; 16 ];
        v.get_block(2);
    }

    #[test]
    fn bit_slicing() {
        let v = vec![ 0b10010110u8, 0b01101001u8 ];
        assert!( !v.get_bit(0) );
        assert!(  v.get_bit(1) );
        assert!(  v.get_bit(2) );
        assert!( !v.get_bit(3) );

        let w = v.bit_slice(2..14);
        assert_eq!( w.bit_len(), 12 );

        assert!(  w.get_bit(0) );
        assert!( !w.get_bit(1) );

        assert_eq!( w.get_bits(2, 4), 0b00001001 );
        assert_eq!( w.get_bits(2, 5), 0b00011001 );
        assert_eq!( w.get_bits(2, 8), 0b10011001 );
        assert_eq!( w.get_bits(3, 8), 0b01001100 );

        assert_eq!( w.get_block(0), 0b01100101 );
        assert_eq!( w.get_block(1), 0b00001010 );
    }

    #[test]
    fn set_block() {
        let mut v = vec![ false; 16 ];

        assert_eq!( v.get_block(0), 0b00000000 );
        assert_eq!( v.get_block(1), 0b00000000 );

        v.set_block(0, 0b10101010 );
        assert_eq!( v.get_block(0), 0b10101010 );
        assert_eq!( v.get_block(1), 0b00000000 );

        v.set_block(1, 0b01010101 );
        assert_eq!( v.get_block(0), 0b10101010 );
        assert_eq!( v.get_block(1), 0b01010101 );

        assert_eq!( v.get_bit(0), false );
        assert_eq!( v.get_bit(1), true );
        assert_eq!( v.get_bit(2), false );
        assert_eq!( v.get_bit(3), true );
    }

    #[test]
    fn set_block_slice() {
        let mut v = vec![ false; 24 ];

        v.as_mut_slice().bit_slice(2..22).set_block(0, 0b11111111);
        assert_eq!( v.get_block(0), 0b11111100 );
        assert_eq!( v.get_block(1), 0b00000011 );
        assert_eq!( v.get_block(2), 0b00000000 );

        v.as_mut_slice().bit_slice(2..22).set_block(1, 0b11111111);
        assert_eq!( v.get_block(0), 0b11111100 );
        assert_eq!( v.get_block(1), 0b11111111 );
        assert_eq!( v.get_block(2), 0b00000011 );

        v.as_mut_slice().bit_slice(2..22).set_block(2, 0b11111111);
        assert_eq!( v.get_block(0), 0b11111100 );
        assert_eq!( v.get_block(1), 0b11111111 );
        assert_eq!( v.get_block(2), 0b00111111 );
    }
}
