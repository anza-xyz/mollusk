use crate::bits_find_lsb::fd_ulong_find_lsb_w_default;
use core::arch::x86_64::{
    __m256i, _mm256_add_epi8, _mm256_and_si256, _mm256_cmpeq_epi64, _mm256_cmpeq_epi8,
    _mm256_cmpgt_epi64, _mm256_cmpgt_epi8, _mm256_extractf128_si256, _mm256_loadu_si256,
    _mm256_movemask_epi8, _mm256_mul_epu32, _mm256_or_si256, _mm256_set1_epi64x, _mm256_set1_epi8,
    _mm256_set_m128i, _mm256_setr_epi64x, _mm256_setr_epi8, _mm256_setzero_si256,
    _mm256_shuffle_epi8, _mm256_slli_epi64, _mm256_slli_si256, _mm256_srli_epi64,
    _mm256_srlv_epi64, _mm256_storeu_si256, _mm256_sub_epi64, _mm256_sub_epi8, _mm_or_si128,
    _mm_setzero_si128, _mm_slli_si128, _mm_srli_si128,
};

#[inline(always)]
pub(crate) fn wuc_ldu(p: *const u8) -> __m256i {
    unsafe { _mm256_loadu_si256(p as *const __m256i) }
}

#[inline(always)]
pub(crate) fn wuc_stu(p: *mut u8, i: __m256i) {
    unsafe { _mm256_storeu_si256(p as *mut __m256i, i) };
}

#[inline(always)]
pub(crate) fn wl(l0: i64, l1: i64, l2: i64, l3: i64) -> __m256i {
    unsafe { _mm256_setr_epi64x(l0, l1, l2, l3) }
}

#[inline(always)]
pub(crate) fn wl_bcast(l0: i64) -> __m256i {
    unsafe { _mm256_set1_epi64x(l0) }
}

#[inline(always)]
pub(crate) fn wl_shl<const IMM8: i32>(a: __m256i) -> __m256i {
    unsafe { _mm256_slli_epi64::<IMM8>(a) }
}

#[inline(always)]
pub(crate) fn wl_shru<const IMM8: i32>(a: __m256i) -> __m256i {
    unsafe { _mm256_srli_epi64::<IMM8>(a) }
}

#[inline(always)]
pub(crate) fn wl_shru_vector(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_srlv_epi64(a, b) }
}

#[inline(always)]
pub(crate) fn wl_and(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_and_si256(a, b) }
}

#[inline(always)]
pub(crate) fn wl_eq(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_cmpeq_epi64(a, b) }
}

#[inline(always)]
pub(crate) fn wl_gt(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_cmpgt_epi64(a, b) }
}

#[inline(always)]
fn wl_sub(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_sub_epi64(a, b) }
}

#[inline(always)]
pub(crate) fn intermediate_to_raw(intermediate: __m256i) -> __m256i {
    /* The computation we need to do here mathematically is
    y=(floor(x/58^k) % 58) for various values of k.  It seems that the
    best way to compute it (at least what the compiler generates in the
    scalar case) is by computing z = floor(x/58^k). y = z -
    58*floor(z/58).  Simplifying, gives, y = floor(x/58^k) -
    58*floor(x/58^(k+1)) (Note, to see that the floors simplify like
    that, represent x in its base58 expansion and then consider that
    dividing by 58^k is just shifting right by k places.) This means we
    can reuse a lot of values!

    We can do the divisions with "magic multiplication" (i.e. multiply
    and shift).  There's a tradeoff between ILP and register pressure
    to make here: we can load a constant for each value of k and just
    compute the division directly, or we could use one constant for
    division by 58 and apply it repeatedly.  I don't know if this is
    optimal, but I use two constants, one for /58 and the other for
    /58^2.  We need to take advantage of the fact the input is
    <58^5<2^32 to produce constants that fit in uints so that we can
    use mul_epu32. */
    let ca = wl_bcast(2369637129);
    let cb = wl_bcast(1307386003);
    let broadcast_58 = wl_bcast(58);
    /* Divide each ulong in r by {58, 58^2=3364}, taking the floor of the
    division.  I used gcc to convert the division to magic
    multiplication. */
    let div3364 = |r: __m256i| wl_shru::<40>(unsafe { _mm256_mul_epu32(wl_shru::<2>(r), cb) });
    /* div(k) stores floor(x/58^k). rem(k) stores div(k) % 58 */
    let div0 = intermediate;
    let div1 = wl_shru::<37>(unsafe { _mm256_mul_epu32(div0, ca) });
    let rem0 = wl_sub(div0, unsafe { _mm256_mul_epu32(div1, broadcast_58) });
    let div2 = div3364(div0);
    let rem1 = wl_sub(div1, unsafe { _mm256_mul_epu32(div2, broadcast_58) });
    let div3 = div3364(div1);
    let rem2 = wl_sub(div2, unsafe { _mm256_mul_epu32(div3, broadcast_58) });
    let div4 = div3364(div2);
    let rem3 = wl_sub(div3, unsafe { _mm256_mul_epu32(div4, broadcast_58) });
    let rem4 = div4;
    /* Okay, we have all 20 terms we need at this point, but they're
    spread out over 5 registers. Each value is stored as an 8B long,
    even though it's less than 58, so 7 of those bytes are 0.  That
    means we're only taking up 4 bytes in each register.  We need to
    get them to a more compact form, but the correct order (in terms of
    place value and recalling where the input vector comes from) is:
    (letters in the right column correspond to diagram below)

       the first value in rem4  (a)
       the first value in rem3  (b)
       ...
       the first value in rem0  (e)
       the second value in rem4 (f)
       ...
       the fourth value in rem0 (t)

    The fact that moves that cross the 128 bit boundary are tricky in
    AVX makes this difficult, forcing an inconvenient output format.

    First, we'll use _mm256_shuffle_epi8 to move the second value in
    each half to byte 5:

      [ a 0 0 0 0 0 0 0  f 0 0 0 0 0 0 0 | k 0 0 0 0 0 0 0  p 0 0 0 0 0 0 0 ] ->
      [ a 0 0 0 0 f 0 0  0 0 0 0 0 0 0 0 | k 0 0 0 0 p 0 0  0 0 0 0 0 0 0 0 ]

    Then for the vectors other than rem4, we'll shuffle them the same
    way, but then shift them left (which corresponds to right in the
    picture...) and OR them together.  */
    let shuffle1 = unsafe {
        _mm256_setr_epi8(
            0, 1, 1, 1, 1, 8, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 8, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1,
        )
    };
    let shift4 = unsafe { _mm256_shuffle_epi8(rem4, shuffle1) };
    let shift3 = unsafe { _mm256_slli_si256(_mm256_shuffle_epi8(rem3, shuffle1), 1) };
    let shift2 = unsafe { _mm256_slli_si256(_mm256_shuffle_epi8(rem2, shuffle1), 2) };
    let shift1 = unsafe { _mm256_slli_si256(_mm256_shuffle_epi8(rem1, shuffle1), 3) };
    let shift0 = unsafe { _mm256_slli_si256(_mm256_shuffle_epi8(rem0, shuffle1), 4) };
    /* The final value is:
    [ a b c d e f g h i j 0 0 0 0 0 0 | k l m n o p q r s t 0 0 0 0 0 0 ]
    */
    unsafe {
        _mm256_or_si256(
            _mm256_or_si256(
                _mm256_or_si256(shift4, shift3),
                _mm256_or_si256(shift2, shift1),
            ),
            shift0,
        )
    }
}

#[inline(always)]
const fn fd_ulong_mask_lsb(n: i32) -> u64 {
    (((n <= 63) as u64) << (n & 63)).wrapping_sub(1)
}

/* Converts each byte in the AVX2 register from raw base58 [0,58) to
base58 digits ('1'-'z', with some skips).  Anything not in the range
[0, 58) will be mapped arbitrarily, but won't affect other bytes. */
#[inline(always)]
pub(crate) fn raw_to_base58(in_: __m256i) -> __m256i {
    /* <30 cycles for two vectors (64 conversions) */
    /* We'll perform the map as an arithmetic expression,
    b58ch(x) = '1' + x + 7*[x>8] + [x>16] + [x>21] + 6*[x>32] + [x>43]
    (using Knuth bracket notation, which maps true/false to 1/0).

    cmpgt uses 0xFF for true and 0x00 for false.  This is very
    convenient, because most of the time we just want to skip one
    character, so we can add 1 by subtracting 0xFF (=-1). */
    let gt0 = unsafe { _mm256_cmpgt_epi8(in_, _mm256_set1_epi8(8)) }; /* skip 7 */
    let gt1 = unsafe { _mm256_cmpgt_epi8(in_, _mm256_set1_epi8(16)) };
    let gt2 = unsafe { _mm256_cmpgt_epi8(in_, _mm256_set1_epi8(21)) };
    let gt3 = unsafe { _mm256_cmpgt_epi8(in_, _mm256_set1_epi8(32)) }; /* skip 6*/
    let gt4 = unsafe { _mm256_cmpgt_epi8(in_, _mm256_set1_epi8(43)) };
    /* Intel doesn't give us an epi8 multiplication instruction, but since
    we know the input is all in {0, -1}, we can just AND both values
    with -7 to get {0, -7}. Similarly for 6. */
    let gt0_7 = unsafe { _mm256_and_si256(gt0, _mm256_set1_epi8(-7)) };
    let gt3_6 = unsafe { _mm256_and_si256(gt3, _mm256_set1_epi8(-6)) };
    let sum = unsafe {
        _mm256_add_epi8(
            _mm256_add_epi8(
                _mm256_add_epi8(_mm256_set1_epi8(-('1' as i8)), gt1), /* Yes, that's the negative character value of '1' */
                _mm256_add_epi8(gt2, gt4),
            ),
            _mm256_add_epi8(gt0_7, gt3_6),
        )
    };
    unsafe { _mm256_sub_epi8(in_, sum) }
}

/* count_leading_zeros_{n} counts the number of zero bytes prior to the
first non-zero byte in the first n bytes.  If all n bytes are zero,
returns n.  Return value is in [0, n].  For the two-vector cases, in0
contains the first 32 bytes and in1 contains the second 32 bytes. */
#[inline(always)]
pub(crate) fn count_leading_zeros_26(in_: __m256i) -> u64 {
    const MASK_LSB_27: u64 = fd_ulong_mask_lsb(27);
    const MASK_LSB_26: u64 = fd_ulong_mask_lsb(26);
    let mask0 = unsafe { _mm256_movemask_epi8(_mm256_cmpeq_epi8(in_, _mm256_setzero_si256())) }
        as u32 as u64;
    let mask = MASK_LSB_27 ^ (mask0 & MASK_LSB_26); /* Flips the low 26 bits and puts a 1 in bit 26 */
    mask.trailing_zeros() as u64
}

#[inline(always)]
pub(crate) fn count_leading_zeros_32(in_: __m256i) -> u64 {
    const MASK_LSB: u64 = fd_ulong_mask_lsb(33);
    let comparison = unsafe { _mm256_cmpeq_epi8(in_, _mm256_setzero_si256()) };
    let xor_rhs = unsafe { _mm256_movemask_epi8(comparison) } as u32 as u64;
    let mask = MASK_LSB ^ xor_rhs;
    mask.trailing_zeros() as u64
}

#[inline(always)]
pub(crate) fn count_leading_zeros_45(in0: __m256i, in1: __m256i) -> u64 {
    const MASK_LSB_46: u64 = fd_ulong_mask_lsb(46);
    const MASK_LSB_13: u64 = fd_ulong_mask_lsb(13);
    let mask0 = unsafe { _mm256_movemask_epi8(_mm256_cmpeq_epi8(in0, _mm256_setzero_si256())) }
        as u32 as u64;
    let mask1 = unsafe { _mm256_movemask_epi8(_mm256_cmpeq_epi8(in1, _mm256_setzero_si256())) }
        as u32 as u64;
    let mask = MASK_LSB_46 ^ (((mask1 & MASK_LSB_13) << 32) | mask0);
    mask.trailing_zeros() as u64
}

#[inline(always)]
pub(crate) fn count_leading_zeros_64(in0: __m256i, in1: __m256i) -> u64 {
    let mask0 = unsafe { _mm256_movemask_epi8(_mm256_cmpeq_epi8(in0, _mm256_setzero_si256())) }
        as u32 as u64;
    let mask1 = unsafe { _mm256_movemask_epi8(_mm256_cmpeq_epi8(in1, _mm256_setzero_si256())) }
        as u32 as u64;
    let mask = !((mask1 << 32) | mask0);
    fd_ulong_find_lsb_w_default(mask, 64) as u64
}

/* ten_per_slot_down_{32,64}: Packs {45,90} raw base58 digits stored in
the bizarre groups of 10 format from intermediate_to_raw into {2,3}
AVX2 registers with the digits stored contiguously. */

/* In this diagram, one letter represents one byte.
[ aaaaaaaaaa000000 bbbbbbbbbb000000 ]
                                    [ cccccccccc000000 dddddddddd000000 ]
                                                                        [ eeeee00000000000 0 ]
[ aaaaaaaaaa000000 ]
[ 0000000000bbbbbb ] ( >> 10B)
                 [ bbbb000000000000 ] (<< 6B)
                 [ 0000cccccccccc00 ] (>> 4B)
                 [ 00000000000000dd ] (>> 14B)
                                    [ dddddddd00000000 ] (<< 2)
                                    [ 00000000eeeee000 ] (>> 8)
    0                   1                   2
In the diagram above, memory addresses increase from left to right.
AVX instructions see the world from a little-endian perspective,
where shifting left by one byte increases the numerical value, which
is equivalent to moving the data one byte later in memory, which
would show in the diagram as moving the values to the right. */

#[inline(always)]
pub(crate) fn ten_per_slot_down_32(in0: __m256i, in1: __m256i, in2: __m256i) -> (__m256i, __m256i) {
    let lo0 = unsafe { _mm256_extractf128_si256(in0, 0) };
    let hi0 = unsafe { _mm256_extractf128_si256(in0, 1) };
    let lo1 = unsafe { _mm256_extractf128_si256(in1, 0) };
    let hi1 = unsafe { _mm256_extractf128_si256(in1, 1) };
    let lo2 = unsafe { _mm256_extractf128_si256(in2, 0) };

    let o0 = unsafe { _mm_or_si128(lo0, _mm_slli_si128(hi0, 10)) };
    let o1 = unsafe {
        _mm_or_si128(
            _mm_or_si128(_mm_srli_si128(hi0, 6), _mm_slli_si128(lo1, 4)),
            _mm_slli_si128(hi1, 14),
        )
    };
    let o2 = unsafe { _mm_or_si128(_mm_srli_si128(hi1, 2), _mm_slli_si128(lo2, 8)) };
    let out0 = unsafe { _mm256_set_m128i(o1, o0) };
    let out1 = unsafe { _mm256_set_m128i(_mm_setzero_si128(), o2) };
    (out0, out1)
}

/* In this diagram, one letter represents one byte.
   (... snip (see diagram above) ... )
    [ eeeeeeeeee000000 ffffffffff000000 ]
                                        [ gggggggggg000000 hhhhhhhhhh000000 ]
                                                                            [ iiiiiiiiii000000 0 ]
    [ 00000000eeeeeeee ] (>> 8)
                     [ ee00000000000000 ] (<< 8)
                     [ 00ffffffffff0000 ] (>> 2)
                     [ 000000000000gggg ] (>> 12)
                                        [ gggggg0000000000 ] (<< 4)
                                        [ 000000hhhhhhhhhh ] (>> 6)
                                                           [ iiiiiiiiii000000 ]
          2               3                   4                   5
*/

#[inline(always)]
pub(crate) fn ten_per_slot_down_64(
    in0: __m256i,
    in1: __m256i,
    in2: __m256i,
    in3: __m256i,
    in4: __m256i,
) -> (__m256i, __m256i, __m256i) {
    let lo0 = unsafe { _mm256_extractf128_si256(in0, 0) };
    let hi0 = unsafe { _mm256_extractf128_si256(in0, 1) };
    let lo1 = unsafe { _mm256_extractf128_si256(in1, 0) };
    let hi1 = unsafe { _mm256_extractf128_si256(in1, 1) };
    let lo2 = unsafe { _mm256_extractf128_si256(in2, 0) };
    let hi2 = unsafe { _mm256_extractf128_si256(in2, 1) };
    let lo3 = unsafe { _mm256_extractf128_si256(in3, 0) };
    let hi3 = unsafe { _mm256_extractf128_si256(in3, 1) };
    let lo4 = unsafe { _mm256_extractf128_si256(in4, 0) };

    let o0 = unsafe { _mm_or_si128(lo0, _mm_slli_si128(hi0, 10)) };
    let o1 = unsafe {
        _mm_or_si128(
            _mm_or_si128(_mm_srli_si128(hi0, 6), _mm_slli_si128(lo1, 4)),
            _mm_slli_si128(hi1, 14),
        )
    };
    let o2 = unsafe { _mm_or_si128(_mm_srli_si128(hi1, 2), _mm_slli_si128(lo2, 8)) };
    let o3 = unsafe {
        _mm_or_si128(
            _mm_or_si128(_mm_srli_si128(lo2, 8), _mm_slli_si128(hi2, 2)),
            _mm_slli_si128(lo3, 12),
        )
    };
    let o4 = unsafe { _mm_or_si128(_mm_srli_si128(lo3, 4), _mm_slli_si128(hi3, 6)) };
    let out0 = unsafe { _mm256_set_m128i(o1, o0) };
    let out1 = unsafe { _mm256_set_m128i(o3, o2) };
    let out2 = unsafe { _mm256_set_m128i(lo4, o4) };
    (out0, out1, out2)
}

/* wl_ld return the 4 longs at the 32-byte aligned / 32-byte sized
location p as a vector long.  wl_ldu is the same but p does not have
to be aligned.  wl_st writes the vector long to the 32-byte aligned /
32-byte sized location p as 4 longs.  wl_stu is the same but p does
not have to be aligned.  In all these 64-bit lane l wlll be at p[l].
FIXME: USE ATTRIBUTES ON P PASSED TO THESE?

Note: gcc knows a __m256i may alias. */
#[inline(always)]
pub(crate) fn wl_ld(p: *const i64) -> __m256i {
    unsafe { core::arch::x86_64::_mm256_load_si256(p as *const __m256i) }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core::arch::x86_64::{_mm256_load_si256, _mm256_store_si256};

    fn wuc_ld(p: *const u8) -> __m256i {
        unsafe { _mm256_load_si256(p as *const __m256i) }
    }

    #[inline(always)]
    fn wuc_st(p: *mut u8, i: __m256i) {
        unsafe { _mm256_store_si256(p as *mut __m256i, i) };
    }

    #[test]
    fn test_fd_ulong_mask_lsb() {
        let w = 64;
        for n in 0..=64 {
            let mask = (if n < w { 1u64 << n } else { 0u64 }).wrapping_sub(1u64);
            assert_eq!(fd_ulong_mask_lsb(n), mask);
        }
    }

    #[repr(C, align(32))]
    #[derive(Debug)]
    struct Aligned32([u8; 64]);

    #[test]
    fn test_count_leading_zeros() {
        let mut buffer = Aligned32([0; 64]);
        let buf_ptr = buffer.0.as_mut_ptr();
        assert_eq!(count_leading_zeros_32(wuc_ld(buf_ptr as *const u8)), 32);
        assert_eq!(
            count_leading_zeros_45(
                wuc_ld(buf_ptr as *const u8),
                wuc_ld(unsafe { buf_ptr.add(32) } as *const u8)
            ),
            45
        );
        unsafe { *buf_ptr = 2 };
        assert_eq!(count_leading_zeros_32(wuc_ld(buf_ptr as *const u8)), 0);
        assert_eq!(
            count_leading_zeros_45(
                wuc_ld(buf_ptr as *const u8),
                wuc_ld(unsafe { buf_ptr.add(32) } as *const u8)
            ),
            0
        );
        unsafe { *buf_ptr = 0 };
        unsafe { *buf_ptr.add(1) = 7 };
        assert_eq!(count_leading_zeros_32(wuc_ld(buf_ptr as *const u8)), 1);
        assert_eq!(
            count_leading_zeros_45(
                wuc_ld(buf_ptr as *const u8),
                wuc_ld(unsafe { buf_ptr.add(32) } as *const u8)
            ),
            1
        );
        unsafe { *buf_ptr.add(1) = 255 };
    }

    #[repr(C, align(32))]
    struct UcharArr32<const N: usize>([u8; N]);

    #[test]
    fn test_ten_per_slot_down_64() {
        let mut in_ = UcharArr32([0; 32 * 5]);
        let mut out = UcharArr32([0; 32 * 3]);
        for i in 0..90 {
            in_.0[16 * (i / 10) + (i % 10)] = i as u8 + 1;
        }
        let in_ptr = in_.0.as_ptr();
        let a = wuc_ld(in_ptr);
        let b = wuc_ld(unsafe { in_ptr.offset(32) });
        let c = wuc_ld(unsafe { in_ptr.offset(64) });
        let d = wuc_ld(unsafe { in_ptr.offset(96) });
        let e = wuc_ld(unsafe { in_ptr.offset(128) });
        let (out0, out1, out2) = ten_per_slot_down_64(a, b, c, d, e);
        let out_ptr = out.0.as_mut_ptr();
        wuc_st(out_ptr, out0);
        wuc_st(unsafe { out_ptr.offset(32) }, out1);
        wuc_st(unsafe { out_ptr.offset(64) }, out2);
        for i in 0..90 {
            assert_eq!(out.0[i], i as u8 + 1);
        }
    }
}
