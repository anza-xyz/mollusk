#![feature(test)]

extern crate bv;
extern crate test;

use bv::BitVec;
use bv::{Bits, BitsExt, BitsMut, BitsMutExt, BitsPush, BitSliceable};

use test::Bencher;
use std::cmp;

const NBITS: usize = 9600;

trait Subject {
    fn new() -> Self;
}

impl Subject for Vec<bool> {
    #[inline(never)]
    fn new() -> Self {
        vec![false; NBITS]
    }
}

impl Subject for Vec<u32> {
    #[inline(never)]
    fn new() -> Self {
        vec![0; NBITS / 32]
    }
}

impl Subject for BitVec<u32> {
    #[inline(never)]
    fn new() -> Self {
        BitVec::new_fill(false, NBITS as u64)
    }
}

type Array<Block> = &'static [Block];

impl Subject for Array<u32> {
    #[inline(never)]
    fn new() -> Self {
        return &[0; NBITS / 32];
    }
}

macro_rules! or3_bench {
    {

        fn $name:ident($v1:ident : &$a1:ty,
                       $v2:ident : &$a2:ty,
                       $v3:ident : &$a3:ty)
                       -> $res:ty {

            $( $body:tt )*
        }

        $( $rest:tt )*

    } =>
    {
        #[bench]
        #[no_mangle]
        fn $name(b: &mut Bencher) {
            fn or3($v1: &$a1, $v2: &$a2, $v3: &$a3) -> $res {
                $( $body )*
            }

            $( $rest )*

            let v1: $a1 = Subject::new();
            let v2: $a2 = Subject::new();
            let v3: $a3 = Subject::new();

            b.iter(|| or3(&v1, &v2, &v3));
        }
    };

    {

        fn $name:ident($v1:ident : &$res:ty, $v2:ident : _) -> _ {
            $( $body:tt )*
        }

        $( $rest:tt )*

    } =>
    {
        or3_bench! {
            fn $name(v1: &$res, v2: &$res, v3: &$res) -> $res {
                or2(&or2(v1, v2), v3)
            }

            fn or2($v1: &$res, $v2: &$res) -> $res {
                $( $body )*
            }

            $( $rest )*
        }
    };
}

or3_bench! {
    fn vec_bool_loop(v1: &Vec<bool>, v2: _) -> _ {
        let len = cmp::min(v1.len(), v2.len());
        let mut result = Vec::with_capacity(len);

        for i in 0 .. len {
            result.push(v1[i] | v2[i]);
        }

        result
    }
}

or3_bench! {
    fn vec_bool_loop_fused(v1: &Vec<bool>, v2: &Vec<bool>, v3: &Vec<bool>) -> Vec<bool> {
        let len = cmp::min(v1.len(), cmp::min(v2.len(), v3.len()));
        let mut result = Vec::with_capacity(len);

        for i in 0 .. len {
            result.push(v1[i] | v2[i] | v3[i]);
        }

        result
    }
}

or3_bench! {
    fn vec_bool_loop_sliced(v1: &Vec<bool>, v2: _) -> _ {
        let len = cmp::min(v1.len(), v2.len());
        let s1  = &v1[..len];
        let s2  = &v2[..len];
        let mut result = s1.to_vec();

        {
            let sr = &mut result[..len];
            for i in 0 .. len {
                sr[i] |= s2[i];
            }
        }

        result
    }
}

or3_bench! {
    fn vec_bool_iter(v1: &Vec<bool>, v2: _) -> _ {
        v1.iter().cloned().zip(v2.iter().cloned())
            .map(|(b1, b2)| b1 | b2)
            .collect()
    }
}

or3_bench! {
    fn vec_bool_loop_fused_sliced(v1: &Vec<bool>, v2: &Vec<bool>, v3: &Vec<bool>) -> Vec<bool> {
        let len = cmp::min(v1.len(), cmp::min(v2.len(), v3.len()));
        let s1 = &v1[.. len];
        let s2 = &v2[.. len];
        let s3 = &v3[.. len];
        let mut result = vec![false; len];

        {
            let sr = &mut result[.. len];
            for i in 0 .. len {
                sr[i] = s1[i] | s2[i] | s3[i];
            }
        }

        result
    }
}

or3_bench! {
    fn vec_bool_loop_fused_sliced_or_assign(v1: &Vec<bool>, v2: &Vec<bool>, v3: &Vec<bool>) -> Vec<bool> {
        let len = cmp::min(v1.len(), cmp::min(v2.len(), v3.len()));
        let s1 = &v1[.. len];
        let s2 = &v2[.. len];
        let s3 = &v3[.. len];
        let mut result = s1.to_vec();

        {
            let sr = &mut result[.. len];
            for i in 0 .. len {
                sr[i] |= s2[i] | s3[i];
            }
        }

        result
    }
}

or3_bench! {
    fn vec_bool_iter_fused(v1: &Vec<bool>, v2: &Vec<bool>, v3: &Vec<bool>) -> Vec<bool> {
        v1.iter().zip(v2.iter()).zip(v3.iter())
            .map(|((b1, b2), b3)| *b1 | *b2 | *b3)
            .collect()
    }
}

or3_bench! {
    fn vec_bool_iter_fused_cloned(v1: &Vec<bool>, v2: &Vec<bool>, v3: &Vec<bool>) -> Vec<bool> {
        v1.iter().cloned().zip(v2.iter().cloned()).zip(v3.iter().cloned())
            .map(|((b1, b2), b3)| b1 | b2 | b3)
            .collect()
    }
}

or3_bench! {
    fn vec_bool_iter_fused_bool_to_int(v1: &Vec<bool>, v2: &Vec<bool>, v3: &Vec<bool>) -> Vec<bool> {
        v1.iter().map(bool_to_int).zip(v2.iter().map(bool_to_int)).zip(v3.iter().map(bool_to_int))
            .map(|((b1, b2), b3)| b1 | b2 | b3 != 0)
            .collect()
    }

    fn bool_to_int(b: &bool) -> u32 {
        if *b {1} else {0}
    }
}

or3_bench! {
    fn vec_u32_bitwise(v1: &Vec<u32>, v2: _) -> _ {
        let block_len = cmp::min(v1.len(), v2.len());
        let mut result = vec![0; block_len];

        for i in 0 .. 32 * block_len as u64 {
            result.set_bit(i, v1.get_bit(i) | v2.get_bit(i));
        }

        result
    }
}

or3_bench! {
    fn vec_u32_bitwise_fused(v1: &Vec<u32>, v2: &Vec<u32>, v3: &Vec<u32>) -> Vec<u32> {
        let block_len = cmp::min(v1.len(), cmp::min(v2.len(), v3.len()));
        let mut result = vec![0; block_len];

        for i in 0 .. 32 * block_len as u64 {
            result.set_bit(i, v1.get_bit(i) | v2.get_bit(i) | v3.get_bit(i));
        }

        result
    }
}

or3_bench! {
    fn vec_u32_loop(v1: &Vec<u32>, v2: _) -> _ {
        let len = cmp::min(v1.len(), v2.len());
        let mut result = Vec::with_capacity(len);

        for i in 0 .. len {
            result.push(v1[i] | v2[i]);
        }

        result
    }
}

or3_bench! {
    fn vec_u32_loop_sliced(v1: &Vec<u32>, v2: _) -> _ {
        let len = cmp::min(v1.len(), v2.len());
        let s1 = &v1[.. len];
        let s2 = &v2[.. len];
        let mut result = s1.to_vec();

        {
            let sr = &mut result[.. len];
            for i in 0 .. len {
                sr[i] |= s2[i];
            }
        }

        result
    }
}

or3_bench! {
    fn vec_u32_loop_fused(v1: &Vec<u32>, v2: &Vec<u32>, v3: &Vec<u32>) -> Vec<u32> {
        let len = cmp::min(v1.len(), cmp::min(v2.len(), v3.len()));
        let mut result = Vec::with_capacity(len);

        for i in 0 .. len {
            result.push(v1[i] | v2[i] | v3[i]);
        }

        result
    }
}

or3_bench! {
    fn vec_u32_loop_fused_assert(v1: &Vec<u32>, v2: &Vec<u32>, v3: &Vec<u32>) -> Vec<u32> {
        let len = v1.len();
        let mut result = v1.to_vec();
        assert_eq!( len, v2.len() );
        assert_eq!( len, v3.len() );
        assert_eq!( len, result.len() );

        for i in 0 .. len {
            result[i] |= v2[i] | v3[i];
        }

        result
    }
}

or3_bench! {
    fn vec_u32_loop_fused_assert_push(v1: &Vec<u32>, v2: &Vec<u32>, v3: &Vec<u32>) -> Vec<u32> {
        let len = v1.len();
        let mut result = Vec::with_capacity(len);
        assert_eq!( len, v1.len() );
        assert_eq!( len, v2.len() );
        assert_eq!( len, v3.len() );
        assert_eq!( len, result.capacity() );

        for i in 0 .. len {
            result.push(v1[i] | v2[i] | v3[i]);
        }

        result
    }
}

or3_bench! {
    fn vec_u32_loop_fused_sliced(v1: &Vec<u32>, v2: &Vec<u32>, v3: &Vec<u32>) -> Vec<u32> {
        let len = cmp::min(v1.len(), cmp::min(v2.len(), v3.len()));
        let s1 = &v1[.. len];
        let s2 = &v2[.. len];
        let s3 = &v3[.. len];
        let mut result = s1.to_vec();

        {
            let sr = &mut result[.. len];
            for i in 0 .. len {
                sr[i] |= s2[i] | s3[i];
            }
        }

        result
    }
}

or3_bench! {
    fn vec_u32_iter(v1: &Vec<u32>, v2: _) -> _ {
        v1.iter().zip(v2.iter())
            .map(|(z1, z2)| *z1 | *z2)
            .collect()
    }
}
or3_bench! {
    fn vec_u32_iter_cloned(v1: &Vec<u32>, v2: _) -> _ {
        v1.iter().cloned().zip(v2.iter().cloned())
            .map(|(z1, z2)| z1 | z2)
            .collect()
    }
}

or3_bench! {
    fn vec_u32_iter_fused(v1: &Vec<u32>, v2: &Vec<u32>, v3: &Vec<u32>) -> Vec<u32> {
        v1.iter().zip(v2.iter()).zip(v3.iter())
            .map(|((z1, z2), z3)| *z1 | *z2 | *z3)
            .collect()
    }
}

or3_bench! {
    fn vec_u32_iter_fused_cloned(v1: &Vec<u32>, v2: &Vec<u32>, v3: &Vec<u32>) -> Vec<u32> {
        v1.iter().cloned().zip(v2.iter().cloned()).zip(v3.iter().cloned())
            .map(|((z1, z2), z3)| z1 | z2 | z3)
            .collect()
    }
}

or3_bench! {
    fn vec_u32_adapter_unfused(v1: &Vec<u32>, v2: &Vec<u32>, v3: &Vec<u32>) -> BitVec<u32> {
        v1.into_bit_or(v2).to_bit_vec().into_bit_or(v3).to_bit_vec()
    }
}

or3_bench! {
    fn vec_u32_adapter(v1: &Vec<u32>, v2: &Vec<u32>, v3: &Vec<u32>) -> BitVec<u32> {
        v1.into_bit_or(v2).into_bit_or(v3).to_bit_vec()
    }
}

or3_bench! {
    fn vec_u32_adapter_or_assign(v1: &Vec<u32>, v2: &Vec<u32>, v3: &Vec<u32>) -> BitVec<u32> {
        let mut result = v1.to_bit_vec();
        result.bit_or_assign(v2.into_bit_or(v3));
        result
    }
}

or3_bench! {
    fn vec_u32_adapter_sliced(v1: &Vec<u32>, v2: &Vec<u32>, v3: &Vec<u32>) -> BitVec<u32> {
        let len = cmp::min(v1.bit_len(), cmp::min(v2.bit_len(), v3.bit_len()));
        let s1 = v1.bit_slice(.. len);
        let s2 = v2.bit_slice(.. len);
        let s3 = v3.bit_slice(.. len);

        s1.into_bit_or(s2).into_bit_or(s3).to_bit_vec()
    }
}

or3_bench! {
    fn vec_u32_adapter_sliced_ragged(v1: &Vec<u32>, v2: &Vec<u32>, v3: &Vec<u32>) -> BitVec<u32> {
        let s1 = v1.bit_slice(..);
        let s2 = v2.bit_slice(..);
        let s3 = v3.bit_slice(..);

        s1.into_bit_or(s2).into_bit_or(s3).to_bit_vec()
    }
}

or3_bench! {
    fn bv_u32_loop(v1: &BitVec<u32>, v2: &BitVec<u32>, v3: &BitVec<u32>) -> BitVec<u32> {
        let len = cmp::min(v1.bit_len(), cmp::min(v2.bit_len(), v3.bit_len()));
        let mut result = BitVec::with_capacity(len);

        for i in 0 .. (len / 32) as usize {
            result.push_block(v1.get_block(i) | v2.get_block(i) | v3.get_block(i));
        }

        result
    }
}

or3_bench! {
    fn bv_u32_loop_sliced(v1: &BitVec<u32>, v2: &BitVec<u32>, v3: &BitVec<u32>) -> BitVec<u32> {
        let len = cmp::min(v1.bit_len(), cmp::min(v2.bit_len(), v3.bit_len()));
        let s1 = v1.bit_slice(.. len);
        let s2 = v2.bit_slice(.. len);
        let s3 = v3.bit_slice(.. len);
        let mut result = BitVec::with_capacity(len);

        for i in 0 .. (len / 32) as usize {
            result.push_block(s1.get_block(i) | s2.get_block(i) | s3.get_block(i));
        }

        result
    }
}

or3_bench! {
    fn bv_u32_adapter(v1: &BitVec<u32>, v2: &BitVec<u32>, v3: &BitVec<u32>) -> BitVec<u32> {
        v1.into_bit_or(v2).into_bit_or(v3).to_bit_vec()
    }
}

or3_bench! {
    fn bv_u32_adapter_sliced(v1: &BitVec<u32>, v2: &BitVec<u32>, v3: &BitVec<u32>) -> BitVec<u32> {
        let len = cmp::min(v1.bit_len(), cmp::min(v2.bit_len(), v3.bit_len()));
        let s1 = v1.bit_slice(.. len);
        let s2 = v2.bit_slice(.. len);
        let s3 = v3.bit_slice(.. len);
        s1.into_bit_or(s2).into_bit_or(s3).to_bit_vec()
    }
}

or3_bench! {
    fn bv_u32_adapter_sliced_offset(v1: &BitVec<u32>, v2: &BitVec<u32>, v3: &BitVec<u32>)
        -> BitVec<u32> {

        let len = cmp::min(v1.bit_len(), cmp::min(v2.bit_len(), v3.bit_len()));
        let s1 = v1.bit_slice(1 .. len);
        let s2 = v2.bit_slice(1 .. len);
        let s3 = v3.bit_slice(1 .. len);

        s1.into_bit_or(s2).into_bit_or(s3).to_bit_vec()
    }
}

or3_bench! {
    fn array_u32_adapter(v1: &Array<u32>, v2: &Array<u32>, v3: &Array<u32>) -> BitVec<u32> {
        v1.into_bit_or(v2).into_bit_or(v3).to_bit_vec()
    }
}

or3_bench! {
    fn array_u32_adapter_sliced(v1: &Array<u32>, v2: &Array<u32>, v3: &Array<u32>) -> BitVec<u32> {
        let len = cmp::min(v1.bit_len(), cmp::min(v2.bit_len(), v3.bit_len()));
        let s1 = v1.bit_slice(.. len);
        let s2 = v2.bit_slice(.. len);
        let s3 = v3.bit_slice(.. len);
        s1.into_bit_or(s2).into_bit_or(s3).to_bit_vec()
    }
}

or3_bench! {
    fn array_u32_loop(v1: &Array<u32>, v2: &Array<u32>, v3: &Array<u32>) -> BitVec<u32> {
        let len = cmp::min(v1.bit_len(), cmp::min(v2.bit_len(), v3.bit_len()));
        let mut result = BitVec::with_capacity(len);

        for i in 0 .. (len / 32) as usize {
            result.push_block(v1.get_block(i) | v2.get_block(i) | v3.get_block(i));
        }

        result
    }
}

or3_bench! {
    fn array_u32_loop_sliced(v1: &Array<u32>, v2: &Array<u32>, v3: &Array<u32>) -> BitVec<u32> {
        let len = cmp::min(v1.bit_len(), cmp::min(v2.bit_len(), v3.bit_len()));
        let s1 = v1.bit_slice(.. len);
        let s2 = v2.bit_slice(.. len);
        let s3 = v3.bit_slice(.. len);
        let mut result = BitVec::with_capacity(len);

        for i in 0 .. (len / 32) as usize {
            result.push_block(s1.get_block(i) | s2.get_block(i) | s3.get_block(i));
        }

        result
    }
}

or3_bench! {
    fn array_u32_loop_sliced_ragged(v1: &Array<u32>, v2: &Array<u32>, v3: &Array<u32>) -> BitVec<u32> {
        let len = cmp::min(v1.bit_len(), cmp::min(v2.bit_len(), v3.bit_len()));
        let s1 = v1.bit_slice(..);
        let s2 = v2.bit_slice(..);
        let s3 = v3.bit_slice(..);
        let mut result = BitVec::with_capacity(len);

        for i in 0 .. (len / 32) as usize {
            result.push_block(s1.get_block(i) | s2.get_block(i) | s3.get_block(i));
        }

        result
    }
}

or3_bench! {
    fn array_u32_adapter_sliced_offset(v1: &Array<u32>, v2: &Array<u32>, v3: &Array<u32>)
        -> BitVec<u32> {

        let len = cmp::min(v1.bit_len(), cmp::min(v2.bit_len(), v3.bit_len()));
        let s1 = v1.bit_slice(1 .. len);
        let s2 = v2.bit_slice(1 .. len);
        let s3 = v3.bit_slice(1 .. len);

        s1.into_bit_or(s2).into_bit_or(s3).to_bit_vec()
    }
}
