/// Like `vec!` but for [`BitVec`](struct.BitVec.html).
///
/// The `bit_vec!` macro creates a `BitVec` literal. It takes two forms:
///
///   - A single `bool`, followed by a semicolon and number of times to repeat. This is
///     equivalent to a call to [`BitVec::new_fill`](struct.BitVec.html#method.new_fill).
///
///   - A sequence of comma-separated `bool`s; this creates a `BitVec` and pushes each `bool` in
///     turn.
///
/// # Examples
///
/// ```
/// # #[macro_use] extern crate bv;
/// use bv::*;
///
/// fn main() {
///     let mut bv1: BitVec = bit_vec![ true; 3 ];
///     let     bv2: BitVec = bit_vec![ true, false, true ];
///
///     assert_ne!(bv1, bv2);
///     bv1.set_bit(1, false);
///     assert_eq!(bv1, bv2);
/// }
/// ```
#[macro_export]
macro_rules! bit_vec {
    ( $e:expr ; $n:expr ) => {
        $crate::BitVec::new_fill($e, $n)
    };

    ( $( $e:expr ),* ) => {
        {
            let mut result = $crate::BitVec::new();
            let _ = &mut result;
            $(
                result.push($e);
            )*
            result
        }
    };

    ( $( $e:expr, )* ) => {
        bit_vec![ $($e),* ]
    };
}

#[test]
fn bit_vec_macro_allows_trailing_comma() {
    let bv1: super::BitVec = bit_vec![true, false, true];
    let bv2: super::BitVec = bit_vec![true, false, true,];
    assert_eq!( bv1, bv2 );
}

#[test]
fn type_1_hygiene() {
    let result = true;
    let bv: super::BitVec = bit_vec![result];
    assert!( bv[0] );
}

// Implements Index for any type that implements Bits.
macro_rules! impl_index_from_bits {
    (
    $(
        impl[ $($param:tt)* ] Index<$ix:ty> for $bv:ty ;
    )+
    )=> {
        $(
            impl<$($param)*> ::std::ops::Index<$ix> for $bv {
                type Output = bool;

                fn index(&self, index: $ix) -> &bool {
                    use $crate::Bits;

                    static TRUE: bool = true;
                    static FALSE: bool = false;

                    if self.get_bit(index) {&TRUE} else {&FALSE}
                }
            }
        )+
    };
}

