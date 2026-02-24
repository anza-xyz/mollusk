use storage::BlockType;
use traits::*;

macro_rules! impl_bits_prim {
    ( $t:ident )
        =>
    {
        impl Bits for $t {
            type Block = $t;

            #[inline]
            fn bit_len(&self) -> u64 {
                Self::nbits() as u64
            }

            #[inline]
            fn block_len(&self) -> usize {
                1
            }

            #[inline]
            fn get_bit(&self, position: u64) -> bool {
                assert!(position < self.bit_len(),
                        "prim::get_bit: out of bounds");
                BlockType::get_bit(*self, position as usize)
            }

            #[inline]
            fn get_block(&self, position: usize) -> Self::Block {
                assert!(position == 0, "prim::get_block: out of bounds");
                *self
            }

            #[inline]
            fn get_bits(&self, start: u64, count: usize) -> Self {
                assert!(start + count as u64 <= Self::nbits() as u64,
                        "prim::get_bits: out of bounds");
                BlockType::get_bits(*self, start as usize, count)
            }
        }

        impl BitsMut for $t {
            #[inline]
            fn set_bit(&mut self, position: u64, value: bool) {
                assert!(position < self.bit_len(),
                        "prim::set_bit: out of bounds");
                *self = self.with_bit(position as usize, value);
            }

            #[inline]
            fn set_block(&mut self, position: usize, value: Self::Block) {
                assert!(position == 0, "prim::set_block: out of bounds");
                *self = value;
            }

            #[inline]
            fn set_bits(&mut self, start: u64, count: usize, value: Self::Block) {
                assert!(start + count as u64 <= Self::nbits() as u64,
                        "prim::set_bits: out of bounds");
                *self = self.with_bits(start as usize, count, value);
            }
        }
    }
}

impl_bits_prim!(u8);
impl_bits_prim!(u16);
impl_bits_prim!(u32);
impl_bits_prim!(u64);
#[cfg(int_128)]
impl_bits_prim!(u128);
impl_bits_prim!(usize);

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn bit_len() {
        assert_eq!( 30u8.bit_len(), 8 );
        assert_eq!( 30u16.bit_len(), 16 );
        assert_eq!( 30u32.bit_len(), 32 );
        assert_eq!( 30u64.bit_len(), 64 );
    }

    #[test]
    fn block_len() {
        assert_eq!( 30u8.block_len(), 1 );
        assert_eq!( 30u16.block_len(), 1 );
        assert_eq!( 30u32.block_len(), 1 );
        assert_eq!( 30u64.block_len(), 1 );
    }

    #[test]
    fn get_bit() {
        assert_eq!( 0b01010101u8.get_bit(0), true );
        assert_eq!( 0b01010101u8.get_bit(1), false );
        assert_eq!( 0b01010101u8.get_bit(2), true );
        assert_eq!( 0b01010101u8.get_bit(3), false );
    }

    quickcheck! {
        fn prop_get_block_u8(value: u8) -> bool {
            value.get_block(0) == value
        }

        fn prop_get_block_u32(value: u32) -> bool {
            value.get_block(0) == value
        }
    }

    #[test]
    fn get_bits() {
        assert_eq!( 0b01010101u8.get_bits(1, 4), 0b00001010 );
        assert_eq!( 0b01010101u8.get_bits(2, 4), 0b00000101 );
    }

    #[test]
    fn set_bit() {
        let mut x = 0b01010101u8;

        x.set_bit(0, false);
        assert_eq!( x, 0b01010100 );

        x.set_bit(1, true);
        assert_eq!( x, 0b01010110 );

        x.set_block(0, 0b00000000);
        assert_eq!( x, 0b00000000 );

        x.set_bits(2, 4, 0b00001111);
        assert_eq!( x, 0b00111100 );
    }

    #[cfg(int_128)]
    #[test]
    fn u128_feature () {
        assert_eq!( 30u128.block_len(), 1 );
    }
}
