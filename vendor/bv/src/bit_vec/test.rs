use super::*;

#[test]
fn bit_slicing() {
    let v: BitVec<u8> = bit_vec![ false, true, true, false, true, false, false, true,
                                  true, false, false, true, false, true, true, false ];
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
fn resize() {
    let mut v: BitVec<u8> = bit_vec![ true; 13 ];
    assert_eq!( v.len(), 13 );

    v.resize(50, false);
    assert_eq!( v.len(), 50 );
    assert_eq!( v.get_bit(12), true );
    assert_eq!( v.get_bit(13), false );
    assert_eq!( v.get_bit(49), false );

    v.resize(67, true);
    assert_eq!( v.len(), 67 );
    assert_eq!( v.get_bit(12), true );
    assert_eq!( v.get_bit(13), false );
    assert_eq!( v.get_bit(49), false );
    assert_eq!( v.get_bit(50), true );
    assert_eq!( v.get_bit(66), true );

    v.set_bit(3, false);
    assert_eq!( v.get_bit(3), false );

    v.resize(17, false);
    assert_eq!( v.len(), 17 );
    assert_eq!( v.get_bit(1), true );
    assert_eq!( v.get_bit(2), true );
    assert_eq!( v.get_bit(3), false );
    assert_eq!( v.get_bit(4), true );
    assert_eq!( v.get_bit(16), false );
}

#[test]
fn shrink_to_fit() {
    let mut v: BitVec<u8> = BitVec::with_capacity(100);
    assert_eq!( v.capacity(), 104 );

    v.push(true);
    v.push(false);
    assert_eq!( v.len(), 2 );
    assert_eq!( v.capacity(), 104 );

    v.shrink_to_fit();
    assert_eq!( v.len(), 2 );
    assert_eq!( v.capacity(), 8 );
}

#[test]
fn into_boxed_slice() {
    let v: BitVec<u8> = bit_vec![ true, false, true ];
    assert_eq!( v.capacity(), 8 );
    let bs = v.into_boxed_slice();
    assert_eq!( bs.len(), 1 );
    assert_eq!( bs[0], 0b00000101 );
}

#[test]
fn truncate() {
    let mut v: BitVec<u8> = BitVec::new_fill(true, 80);
    assert_eq!( v.len(), 80 );
    assert_eq!( v.get_bit(34), true );

    v.truncate(45);
    assert_eq!( v.len(), 45 );
    assert_eq!( v.get_bit(34), true );
}

#[test]
fn as_mut_slice() {
    let mut v: BitVec<u8> = BitVec::new_fill(true, 77);
    let w = v.as_mut_slice();
    assert_eq!( w.len(), 77 );
    assert_eq!( w.get_block(0), 0b11111111 );
}

#[test]
fn pop() {
    let mut v: BitVec<u8> = bit_vec![true, false, true];
    assert_eq!( v.pop(), Some(true) );
    assert_eq!( v.pop(), Some(false) );
    assert_eq!( v.pop(), Some(true) );
    assert_eq!( v.pop(), None );
}

#[test]
fn clear_and_is_empty() {
    let mut v: BitVec<u8> = bit_vec![true, false, true];
    assert_eq!( v.len(), 3 );
    assert!( !v.is_empty() );
    v.clear();
    assert_eq!( v.len(), 0 );
    assert!( v.is_empty() );
}

#[test]
fn push_bit_and_pop_bit() {
    let mut v: BitVec<u8> = BitVec::new();
    v.push_bit(true);
    v.push_bit(false);
    assert_eq!( v.pop_bit(), Some(false) );
    assert_eq!( v.pop_bit(), Some(true) );
    assert_eq!( v.pop_bit(), None );
}

#[test]
fn set_through_slice() {
    let mut v: BitVec<u8> = bit_vec![true, false, true];

    {
        let mut w = v.as_mut_slice().bit_slice(1..2);
        assert_eq!(w.get_block(0), 0);
        w.set_bit(0, true);
    }

    assert_eq!(v, bit_vec![true, true, true] );
}

#[test]
fn set_bits_one_block_fastpath() {
    let mut v: BitVec<u8> = bit_vec![false; 8];
    v.set_bits(2, 4, 0b1111);
    assert_eq!( v.get_block(0), 0b00111100 );
}

#[test]
fn from_bits() {
    let mut bits = vec![true; 20];
    bits[3] = false;
    let bv = BitVec::from_bits(&bits);
    assert_eq!( bv.len(), 20 );
    assert!(  bv[0] );
    assert!(  bv[1] );
    assert!(  bv[2] );
    assert!( !bv[3] );
    assert!(  bv[4] );
    assert!(  bv[19] );
}

#[test]
fn from_bits_slice() {
    let mut bits: BitVec = bit_vec![true; 20];
    bits.set_bit(3, false);
    let slice = bits.bit_slice(1..);
    let bv = BitVec::from_bits(&slice);
    assert_eq!( bv.len(), 19 );
    assert!(  bv[0] );
    assert!(  bv[1] );
    assert!( !bv[2] );
    assert!(  bv[3] );
    assert!(  bv[18] );
}

#[test]
fn disequality() {
    let bv1: BitVec = bit_vec![true, true, false];
    let bv2         = bit_vec![true, true];
    assert_ne!( bv1, bv2 );
}

#[test]
fn mixed_equality() {
    let bv: BitVec<u8> = bit_vec![true, false, true];
    let array: &[bool] = &[true, false, true];
    assert_eq!( bv, array );
}
