use five8_core::{
    BASE58_ENCODED_32_MAX_LEN, BASE58_ENCODED_64_MAX_LEN, BINARY_SZ_32, BINARY_SZ_64,
    INTERMEDIATE_SZ_32, INTERMEDIATE_SZ_64, N_32, N_64, RAW58_SZ_32, RAW58_SZ_64,
};
#[cfg(target_feature = "avx2")]
use {
    crate::avx::{
        count_leading_zeros_26, count_leading_zeros_32, count_leading_zeros_45,
        count_leading_zeros_64, intermediate_to_raw, raw_to_base58, ten_per_slot_down_32,
        ten_per_slot_down_64, wl, wl_and, wl_bcast, wl_eq, wl_gt, wl_ld, wl_shl, wl_shru,
        wl_shru_vector, wuc_ldu, wuc_stu,
    },
    core::arch::x86_64::{
        __m128i, __m256i, _mm256_extractf128_si256, _mm256_loadu_si256, _mm256_maskstore_epi64,
        _mm256_set_epi8, _mm256_shuffle_epi8, _mm_bslli_si128, _mm_storeu_si128,
    },
};

#[cfg(any(not(target_feature = "avx2"), feature = "dev-utils"))]
const BASE58_CHARS: [u8; 58] = [
    b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'A', b'B', b'C', b'D', b'E', b'F', b'G',
    b'H', b'J', b'K', b'L', b'M', b'N', b'P', b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y',
    b'Z', b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j', b'k', b'm', b'n', b'o', b'p',
    b'q', b'r', b's', b't', b'u', b'v', b'w', b'x', b'y', b'z',
];

const R1DIV: u64 = 656356768u64;

/* Contains the unique values less than 58^5 such that:
  2^(32*(7-j)) = sum_k table[j][k]*58^(5*(7-k))

The second dimension of this table is actually ceil(log_(58^5)
(2^(32*(BINARY_SZ-1))), but that's almost always INTERMEDIATE_SZ-1 */

const ENC_TABLE_32: [[u32; INTERMEDIATE_SZ_32 - 1]; BINARY_SZ_32] = [
    [
        513735, 77223048, 437087610, 300156666, 605448490, 214625350, 141436834, 379377856,
    ],
    [
        0, 78508, 646269101, 118408823, 91512303, 209184527, 413102373, 153715680,
    ],
    [
        0, 0, 11997, 486083817, 3737691, 294005210, 247894721, 289024608,
    ],
    [0, 0, 0, 1833, 324463681, 385795061, 551597588, 21339008],
    [0, 0, 0, 0, 280, 127692781, 389432875, 357132832],
    [0, 0, 0, 0, 0, 42, 537767569, 410450016],
    [0, 0, 0, 0, 0, 0, 6, 356826688],
    [0, 0, 0, 0, 0, 0, 0, 1],
];

const ENC_TABLE_64: [[u32; INTERMEDIATE_SZ_64 - 1]; BINARY_SZ_64] = [
    [
        2631, 149457141, 577092685, 632289089, 81912456, 221591423, 502967496, 403284731,
        377738089, 492128779, 746799, 366351977, 190199623, 38066284, 526403762, 650603058,
        454901440,
    ],
    [
        0, 402, 68350375, 30641941, 266024478, 208884256, 571208415, 337765723, 215140626,
        129419325, 480359048, 398051646, 635841659, 214020719, 136986618, 626219915, 49699360,
    ],
    [
        0, 0, 61, 295059608, 141201404, 517024870, 239296485, 527697587, 212906911, 453637228,
        467589845, 144614682, 45134568, 184514320, 644355351, 104784612, 308625792,
    ],
    [
        0, 0, 0, 9, 256449755, 500124311, 479690581, 372802935, 413254725, 487877412, 520263169,
        176791855, 78190744, 291820402, 74998585, 496097732, 59100544,
    ],
    [
        0, 0, 0, 0, 1, 285573662, 455976778, 379818553, 100001224, 448949512, 109507367, 117185012,
        347328982, 522665809, 36908802, 577276849, 64504928,
    ],
    [
        0, 0, 0, 0, 0, 0, 143945778, 651677945, 281429047, 535878743, 264290972, 526964023,
        199595821, 597442702, 499113091, 424550935, 458949280,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 21997789, 294590275, 148640294, 595017589, 210481832, 404203788,
        574729546, 160126051, 430102516, 44963712,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 3361701, 325788598, 30977630, 513969330, 194569730, 164019635,
        136596846, 626087230, 503769920,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 513735, 77223048, 437087610, 300156666, 605448490, 214625350,
        141436834, 379377856,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 78508, 646269101, 118408823, 91512303, 209184527, 413102373,
        153715680,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 11997, 486083817, 3737691, 294005210, 247894721, 289024608,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1833, 324463681, 385795061, 551597588, 21339008,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 280, 127692781, 389432875, 357132832,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 537767569, 410450016,
    ],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 356826688],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
];

#[cfg(target_feature = "avx2")]
const fn fd_ulong_align_up(x: usize, a: usize) -> usize {
    ((x) + ((a) - 1)) & (!((a) - 1))
}

#[cfg(target_feature = "avx2")]
const INTERMEDIATE_SZ_W_PADDING_32: usize = fd_ulong_align_up(INTERMEDIATE_SZ_32, 4);
#[cfg(target_feature = "avx2")]
const INTERMEDIATE_SZ_W_PADDING_64: usize = fd_ulong_align_up(INTERMEDIATE_SZ_64, 4);

#[cfg(not(target_feature = "avx2"))]
const INTERMEDIATE_SZ_W_PADDING_32: usize = INTERMEDIATE_SZ_32;
#[cfg(not(target_feature = "avx2"))]
const INTERMEDIATE_SZ_W_PADDING_64: usize = INTERMEDIATE_SZ_64;

#[cfg_attr(target_feature = "avx2", repr(align(32)))]
#[repr(C)]
struct Intermediate<const INTERMEDIATE_SZ_W_PADDING: usize>([u64; INTERMEDIATE_SZ_W_PADDING]);

#[cfg(feature = "dev-utils")]
#[repr(transparent)]
pub struct IntermediatePub<const INTERMEDIATE_SZ_W_PADDING: usize>(
    Intermediate<INTERMEDIATE_SZ_W_PADDING>,
);

#[cfg(target_feature = "avx2")]
#[inline(always)]
fn in_leading_0s_32_avx(bytes: *const u8) -> u64 {
    let bytes_ = wuc_ldu(bytes);
    count_leading_zeros_32(bytes_)
}

#[cfg(target_feature = "avx2")]
#[inline(always)]
fn in_leading_0s_64_avx(bytes: *const u8) -> u64 {
    let bytes_0 = wuc_ldu(bytes);
    let bytes_1 = wuc_ldu(unsafe { bytes.offset(32) });
    count_leading_zeros_64(bytes_0, bytes_1)
}

#[cfg(any(not(target_feature = "avx2"), feature = "dev-utils"))]
#[inline(always)]
fn in_leading_0s_scalar<const BYTE_CNT: usize>(bytes: *const u8) -> u64 {
    let mut in_leading_0s = 0;
    while in_leading_0s < BYTE_CNT {
        if unsafe { bytes.add(in_leading_0s).read() != 0 } {
            break;
        }
        in_leading_0s += 1;
    }
    in_leading_0s as u64
}

#[cfg(feature = "dev-utils")]
pub fn in_leading_0s_scalar_pub<const BYTE_CNT: usize>(bytes: *const u8) -> u64 {
    in_leading_0s_scalar::<BYTE_CNT>(bytes)
}

#[cfg(feature = "dev-utils")]
pub fn in_leading_0s_32_pub(bytes: *const u8) -> u64 {
    #[cfg(target_feature = "avx2")]
    {
        in_leading_0s_32_avx(bytes)
    }
    #[cfg(not(target_feature = "avx2"))]
    {
        in_leading_0s_scalar::<N_32>(bytes)
    }
}

#[inline(always)]
fn add_binary_to_intermediate<const INTERMEDIATE_SZ_W_PADDING: usize, const BINARY_SZ: usize>(
    intermediate: &mut Intermediate<INTERMEDIATE_SZ_W_PADDING>,
    j: usize,
    binary: [u32; BINARY_SZ],
    i: usize,
    multiplier: u64,
) {
    unsafe {
        *intermediate.0.get_unchecked_mut(j + 1) += *binary.get_unchecked(i) as u64 * multiplier;
    }
}

#[cfg(any(not(target_feature = "avx2"), feature = "dev-utils"))]
#[inline(always)]
fn intermediate_to_base58_scalar<
    const INTERMEDIATE_SZ_W_PADDING: usize,
    const RAW58_SZ: usize,
    const INTERMEDIATE_SZ: usize,
>(
    intermediate: &Intermediate<INTERMEDIATE_SZ_W_PADDING>,
    in_leading_0s: u64,
    out: &mut [u8],
) -> usize {
    /* Convert intermediate form to base 58.  This form of conversion
    exposes tons of ILP, but it's more than the CPU can take advantage
    of.
      X = sum_i raw_base58[i] * 58^(RAW58_SZ-1-i) */
    let mut raw_base58 = [0u8; RAW58_SZ];
    for i in 0..INTERMEDIATE_SZ {
        /* We know intermediate[ i ] < 58^5 < 2^32 for all i, so casting to
        a uint is safe.  GCC doesn't seem to be able to realize this, so
        when it converts ulong/ulong to a magic multiplication, it
        generates the single-op 64b x 64b -> 128b mul instruction.  This
        hurts the CPU's ability to take advantage of the ILP here. */
        let v = unsafe { *intermediate.0.get_unchecked(i) } as u32;
        unsafe {
            *raw_base58.get_unchecked_mut(5 * i + 4) = (v % 58) as u8;
        }
        unsafe {
            *raw_base58.get_unchecked_mut(5 * i + 3) = ((v / 58) % 58) as u8;
        }
        unsafe {
            *raw_base58.get_unchecked_mut(5 * i + 2) = ((v / 3364) % 58) as u8;
        }
        unsafe {
            *raw_base58.get_unchecked_mut(5 * i + 1) = ((v / 195112) % 58) as u8;
        }
        unsafe {
            *raw_base58.get_unchecked_mut(5 * i) = (v / 11316496) as u8;
        } /* We know this one is less than 58 */
    }
    /* Finally, actually convert to the string.  We have to ignore all the
    leading zeros in raw_base58 and instead insert in_leading_0s
    leading '1' characters.  We can show that raw_base58 actually has
    at least in_leading_0s, so we'll do this by skipping the first few
    leading zeros in raw_base58. */
    let mut raw_leading_0s = 0;
    while raw_leading_0s < RAW58_SZ {
        if raw_base58[raw_leading_0s] != 0 {
            break;
        }
        raw_leading_0s += 1;
    }
    /* It's not immediately obvious that raw_leading_0s >= in_leading_0s,
    but it's true.  In base b, X has floor(log_b X)+1 digits.  That
    means in_leading_0s = N-1-floor(log_256 X) and raw_leading_0s =
    RAW58_SZ-1-floor(log_58 X).  Let X<256^N be given and consider:

    raw_leading_0s - in_leading_0s =
      =  RAW58_SZ-N + floor( log_256 X ) - floor( log_58 X )
      >= RAW58_SZ-N - 1 + ( log_256 X - log_58 X ) .

    log_256 X - log_58 X is monotonically decreasing for X>0, so it
    achieves it minimum at the maximum possible value for X, i.e.
    256^N-1.
      >= RAW58_SZ-N-1 + log_256(256^N-1) - log_58(256^N-1)

    When N==32, RAW58_SZ is 45, so this gives skip >= 0.29
    When N==64, RAW58_SZ is 90, so this gives skip >= 1.59.

    Regardless, raw_leading_0s - in_leading_0s >= 0. */
    let skip = raw_leading_0s - in_leading_0s as usize;
    for i in 0..(RAW58_SZ - skip) {
        unsafe {
            *out.get_unchecked_mut(i) = BASE58_CHARS[raw_base58[skip + i] as usize];
        }
    }
    skip
}

#[cfg(feature = "dev-utils")]
pub fn intermediate_to_base58_scalar_64_pub(
    intermediate: &IntermediatePub<INTERMEDIATE_SZ_W_PADDING_64>,
    in_leading_0s: u64,
    out: &mut [u8],
) -> usize {
    intermediate_to_base58_scalar::<INTERMEDIATE_SZ_W_PADDING_64, RAW58_SZ_64, INTERMEDIATE_SZ_64>(
        &intermediate.0,
        in_leading_0s,
        out,
    )
}

#[cfg(feature = "dev-utils")]
pub fn intermediate_to_base58_32_pub(
    intermediate: &IntermediatePub<INTERMEDIATE_SZ_W_PADDING_32>,
    in_leading_0s: u64,
    out: &mut [u8],
) -> usize {
    intermediate_to_base58_32(&intermediate.0, in_leading_0s, out)
}

#[inline(always)]
fn adjust_intermediate_array<
    const INTERMEDIATE_SZ_W_PADDING: usize,
    const INTERMEDIATE_SZ: usize,
>(
    intermediate: &mut Intermediate<INTERMEDIATE_SZ_W_PADDING>,
) {
    /* Now we make sure each term is less than 58^5. Again, we have to be
    a bit careful of overflow.

    For N==32, in the worst case, as before, intermediate[8] will be
    just over 2^63 and intermediate[7] will be just over 2^62.6.  In
    the first step, we'll add floor(intermediate[8]/58^5) to
    intermediate[7].  58^5 is pretty big though, so intermediate[7]
    barely budges, and this is still fine.

    For N==64, in the worst case, the biggest entry in intermediate at
    this point is 2^63.87, and in the worst case, we add (2^64-1)/58^5,
    which is still about 2^63.87. */
    for i in (1..=INTERMEDIATE_SZ - 1).rev() {
        unsafe {
            *intermediate.0.get_unchecked_mut(i - 1) += *intermediate.0.get_unchecked(i) / R1DIV;
        }
        unsafe {
            *intermediate.0.get_unchecked_mut(i) %= R1DIV;
        }
    }
}

#[inline(always)]
fn init_intermediate_array<const INTERMEDIATE_SZ_W_PADDING: usize>(
) -> Intermediate<INTERMEDIATE_SZ_W_PADDING> {
    Intermediate([0u64; INTERMEDIATE_SZ_W_PADDING])
}

#[cfg(target_feature = "avx2")]
#[inline(always)]
fn u8s_to_u32s_swapped_mask_32() -> __m256i {
    unsafe {
        _mm256_set_epi8(
            12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3, 12, 13, 14, 15, 8, 9, 10, 11, 4,
            5, 6, 7, 0, 1, 2, 3,
        )
    }
}

#[cfg(not(target_feature = "avx2"))]
#[inline(always)]
fn u8s_to_u32s_scalar<const N: usize, const BINARY_SZ: usize>(
    out: &mut [u8; N],
    binary_u8: &[u8; N],
) {
    for i in 0..BINARY_SZ {
        let idx = i * 4;
        #[cfg(target_endian = "little")]
        unsafe {
            *out.get_unchecked_mut(idx) = *binary_u8.get_unchecked(idx + 3);
            *out.get_unchecked_mut(idx + 1) = *binary_u8.get_unchecked(idx + 2);
            *out.get_unchecked_mut(idx + 2) = *binary_u8.get_unchecked(idx + 1);
            *out.get_unchecked_mut(idx + 3) = *binary_u8.get_unchecked(idx);
        }
        #[cfg(target_endian = "big")]
        unsafe {
            *out.get_unchecked_mut(idx) = *binary_u8.get_unchecked(idx);
            *out.get_unchecked_mut(idx + 1) = *binary_u8.get_unchecked(idx + 1);
            *out.get_unchecked_mut(idx + 2) = *binary_u8.get_unchecked(idx + 2);
            *out.get_unchecked_mut(idx + 3) = *binary_u8.get_unchecked(idx + 3);
        }
    }
}

#[cfg(target_feature = "avx2")]
#[inline(always)]
fn u8s_to_u32s_swapped_32_register(bytes: &[u8; N_32]) -> __m256i {
    let mask = u8s_to_u32s_swapped_mask_32();
    unsafe { _mm256_shuffle_epi8(_mm256_loadu_si256(bytes.as_ptr() as *const __m256i), mask) }
}

#[cfg(target_feature = "avx2")]
#[inline(always)]
fn u8s_to_u32s_swapped_64_register(bytes: &[u8; N_64]) -> [__m256i; 2] {
    let mask = u8s_to_u32s_swapped_mask_32();
    let first_load =
        unsafe { _mm256_loadu_si256(bytes.get_unchecked(..32).as_ptr() as *const __m256i) };
    let first = unsafe { _mm256_shuffle_epi8(first_load, mask) };
    [first, unsafe {
        _mm256_shuffle_epi8(
            _mm256_loadu_si256(bytes.get_unchecked(32..64).as_ptr() as *const __m256i),
            mask,
        )
    }]
}

#[cfg(target_feature = "avx2")]
#[inline(always)]
fn u8s_to_u32s_swapped_32(bytes: &[u8; N_32], out: &mut [u8; N_32]) {
    let res_m256i = u8s_to_u32s_swapped_32_register(bytes);
    let out_bytes: [u8; N_32] = unsafe { core::mem::transmute(res_m256i) };
    *out = out_bytes;
}

// replacing this func with the scalar version worsened the
// encode_64 benchmark by 150%.
#[cfg(target_feature = "avx2")]
#[inline(always)]
fn u8s_to_u32s_swapped_64(bytes: &[u8; N_64], out: &mut [u8; N_64]) {
    let res_nested = u8s_to_u32s_swapped_64_register(bytes);
    let out_bytes: [u8; N_64] = unsafe { core::mem::transmute(res_nested) };
    *out = out_bytes;
}

// fn u8s_to_u32s_swapped_64(bytes: [u8; N_64]) -> [u8; N_64] {
//     // this fails with SIGILL: illegal instruction. Maybe some day it won't.
//     let mask = unsafe {
//         core::arch::x86_64::_mm512_set_epi8(
//             60, 61, 62, 63, 56, 57, 58, 59, 52, 53, 54, 55, 48, 49, 50, 51, 44, 45, 46, 47, 40, 41,
//             42, 43, 36, 37, 38, 39, 32, 33, 34, 35, 28, 29, 30, 31, 24, 25, 26, 27, 20, 21, 22, 23,
//             16, 17, 18, 19, 12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3,
//         )
//     };
//     std::println!("mask: {mask:?}");
//     let res_m256i =
//         unsafe { core::arch::x86_64::_mm512_shuffle_epi8(core::mem::transmute(bytes), mask) };
//     unsafe { core::mem::transmute(res_m256i) }
// }

#[inline(always)]
fn make_binary_array_32(bytes: &[u8; N_32]) -> [u32; BINARY_SZ_32] {
    // on LE take four-byte blocks and reverse them
    // 3 2 1 0 7 6 5 4 etc
    // on BE just take four-byte blocks

    let mut out = [0u8; N_32];
    #[cfg(target_feature = "avx2")]
    {
        u8s_to_u32s_swapped_32(bytes, &mut out);
    }
    #[cfg(not(target_feature = "avx2"))]
    {
        u8s_to_u32s_scalar::<N_32, BINARY_SZ_32>(&mut out, bytes);
    }
    unsafe { core::mem::transmute(out) }
}

#[inline(always)]
fn make_binary_array_64(bytes: &[u8; N_64]) -> [u32; BINARY_SZ_64] {
    // on LE take four-byte blocks and reverse them
    // 3 2 1 0 7 6 5 4 etc
    // on BE this is a noop

    let mut out = [0u8; N_64];
    #[cfg(target_feature = "avx2")]
    {
        u8s_to_u32s_swapped_64(bytes, &mut out);
    }
    #[cfg(not(target_feature = "avx2"))]
    {
        u8s_to_u32s_scalar::<N_64, BINARY_SZ_64>(&mut out, bytes);
    }
    unsafe { core::mem::transmute(out) }
}

#[cfg(feature = "dev-utils")]
pub fn make_binary_array_32_pub(bytes: &[u8; N_32]) -> [u32; BINARY_SZ_32] {
    make_binary_array_32(bytes)
}

#[cfg(feature = "dev-utils")]
pub fn make_binary_array_64_pub(bytes: &[u8; N_64]) -> [u32; BINARY_SZ_64] {
    make_binary_array_64(bytes)
}

/// Encode a 64-byte array.
///
/// Mutates the provided `out` array and returns a u8 `len`
/// which indicates how many bytes of the `out` array were actually written to.
/// The remaining bytes are unchanged, hence the result we care about after
/// calling the function is `out[..len as usize]`.
///
/// # Examples
/// ```
/// let mut buf = [0u8; 88];
/// let bytes = &[
///     0, 0, 10, 85, 198, 191, 71, 18, 5, 54, 6, 255, 181, 32, 227, 150, 208, 3, 157, 135, 222,
///     67, 50, 23, 237, 51, 240, 123, 34, 148, 111, 84, 98, 162, 236, 133, 31, 93, 185, 142, 108,
///     41, 191, 1, 138, 6, 192, 0, 46, 93, 25, 65, 243, 223, 225, 225, 85, 55, 82, 251, 109, 132,
///     165, 2,
/// ];
/// let len = five8::encode_64(bytes, &mut buf);
/// assert_eq!(
///     &buf[..len as usize],
///     [
///         49, 49, 99, 103, 84, 72, 52, 68, 53, 101, 56, 83, 51, 115, 110, 68, 52, 52, 52, 87, 98,
///         98, 71, 114, 107, 101, 112, 106, 84, 118, 87, 77, 106, 50, 106, 107, 109, 67, 71, 74, 116,
///         103, 110, 51, 72, 55, 113, 114, 80, 98, 49, 66, 110, 119, 97, 112, 120, 112, 98, 71, 100,
///         82, 116, 72, 81, 104, 57, 116, 57, 87, 98, 110, 57, 116, 54, 90, 68, 71, 72, 122, 87, 112,
///         76, 52, 100, 102
///     ]
/// );
/// assert_eq!(len, 86);
#[inline]
pub fn encode_64(bytes: &[u8; N_64], out: &mut [u8; BASE58_ENCODED_64_MAX_LEN]) -> u8 {
    let bytes_ptr = bytes as *const u8;
    let in_leading_0s = {
        #[cfg(target_feature = "avx2")]
        {
            in_leading_0s_64_avx(bytes_ptr)
        }
        #[cfg(not(target_feature = "avx2"))]
        {
            in_leading_0s_scalar::<N_64>(bytes_ptr)
        }
    };
    let binary = make_binary_array_64(bytes);
    let intermediate = make_intermediate_array_64(binary);
    let skip = {
        #[cfg(not(target_feature = "avx2"))]
        {
            intermediate_to_base58_scalar::<
                INTERMEDIATE_SZ_W_PADDING_64,
                RAW58_SZ_64,
                INTERMEDIATE_SZ_64,
            >(&intermediate, in_leading_0s, out)
        }
        #[cfg(target_feature = "avx2")]
        {
            let intermediate_ptr = intermediate.0.as_ptr() as *const i64;
            let raw0 = intermediate_to_raw(wl_ld(intermediate_ptr));
            let raw1 = intermediate_to_raw(wl_ld(unsafe { intermediate_ptr.offset(4) }));
            let raw2 = intermediate_to_raw(wl_ld(unsafe { intermediate_ptr.offset(8) }));
            let raw3 = intermediate_to_raw(wl_ld(unsafe { intermediate_ptr.offset(12) }));
            let raw4 = intermediate_to_raw(wl_ld(unsafe { intermediate_ptr.offset(16) }));
            let (compact0, compact1, compact2) = ten_per_slot_down_64(raw0, raw1, raw2, raw3, raw4);
            let raw_leading_0s_part1 = count_leading_zeros_64(compact0, compact1);
            let raw_leading_0s_part2 = count_leading_zeros_26(compact2);
            let raw_leading_0s = if raw_leading_0s_part1 < 64 {
                raw_leading_0s_part1
            } else {
                64 + raw_leading_0s_part2
            };
            let base58_0 = raw_to_base58(compact0);
            let base58_1 = raw_to_base58(compact1);
            let base58_2 = raw_to_base58(compact2);
            let skip = raw_leading_0s - in_leading_0s;
            /* We'll do something similar.  The final string is between 64 and 88
            characters, so skip is [2, 26].
            */
            let w_skip = wl_bcast(skip as i64);
            let mod8_mask = wl_bcast(7);
            let compare = wl(0, 1, 2, 3);
            let shift_qty = wl_shl::<3>(wl_and(w_skip, mod8_mask)); /* bytes->bits */
            let shifted = wl_shru_vector(base58_0, shift_qty);
            let skip_div8 = wl_shru::<3>(w_skip);
            let mask1 = wl_eq(skip_div8, compare);
            let mask2 = wl_gt(compare, skip_div8);
            let out_ptr = out.as_mut_ptr();
            unsafe {
                _mm256_maskstore_epi64(
                    (out_ptr.offset(-8 * (skip as isize / 8))) as *mut i64,
                    mask1,
                    shifted,
                )
            };
            unsafe {
                _mm256_maskstore_epi64(
                    out_ptr.offset(-(skip as isize)) as *mut i64,
                    mask2,
                    base58_0,
                )
            };

            unsafe { wuc_stu(out_ptr.offset(32 - skip as isize), base58_1) };

            let last = unsafe { _mm_bslli_si128(_mm256_extractf128_si256(base58_2, 1), 6) };
            unsafe {
                _mm_storeu_si128(
                    out_ptr.offset(64 + 16 - 6 - skip as isize) as *mut __m128i,
                    last,
                )
            };
            unsafe {
                _mm_storeu_si128(
                    out_ptr.offset(64 - skip as isize) as *mut __m128i,
                    _mm256_extractf128_si256(base58_2, 0),
                )
            };
            skip
        }
    };
    RAW58_SZ_64 as u8 - skip as u8
}

#[inline(always)]
fn make_intermediate_array_64(
    binary: [u32; BINARY_SZ_64],
) -> Intermediate<INTERMEDIATE_SZ_W_PADDING_64> {
    /* Convert to the intermediate format:
      X = sum_i intermediate[i] * 58^(5*(INTERMEDIATE_SZ-1-i))
    Initially, we don't require intermediate[i] < 58^5, but we do want
    to make sure the sums don't overflow. */
    let mut intermediate = init_intermediate_array::<INTERMEDIATE_SZ_W_PADDING_64>();

    /* If we do it the same way as the 32B conversion, intermediate[16]
    can overflow when the input is sufficiently large.  We'll do a
    mini-reduction after the first 8 steps.  After the first 8 terms,
    the largest intermediate[16] can be is 2^63.87.  Then, after
    reduction it'll be at most 58^5, and after adding the last terms,
    it won't exceed 2^63.1.  We do need to be cautious that the
    mini-reduction doesn't cause overflow in intermediate[15] though.
    Pre-mini-reduction, it's at most 2^63.05.  The mini-reduction adds
    at most 2^64/58^5, which is negligible.  With the final terms, it
    won't exceed 2^63.69, which is fine. Other terms are less than
    2^63.76, so no problems there. */
    for i in 0..8 {
        for j in 0..INTERMEDIATE_SZ_64 - 1 {
            let multiplier = unsafe { *ENC_TABLE_64.get_unchecked(i).get_unchecked(j) } as u64;
            add_binary_to_intermediate(&mut intermediate, j, binary, i, multiplier);
        }
    }
    /* Mini-reduction */
    unsafe {
        *intermediate.0.get_unchecked_mut(15) += intermediate.0.get_unchecked(16) / R1DIV;
    }
    unsafe {
        *intermediate.0.get_unchecked_mut(16) %= R1DIV;
    }
    /* Finish iterations */
    for i in 8..BINARY_SZ_64 {
        for j in 0..INTERMEDIATE_SZ_64 - 1 {
            let multiplier = unsafe { *ENC_TABLE_64.get_unchecked(i).get_unchecked(j) as u64 };
            add_binary_to_intermediate(&mut intermediate, j, binary, i, multiplier);
        }
    }
    adjust_intermediate_array::<INTERMEDIATE_SZ_W_PADDING_64, INTERMEDIATE_SZ_64>(
        &mut intermediate,
    );
    intermediate
}

#[cfg(feature = "dev-utils")]
pub fn make_intermediate_array_32_pub(
    binary: [u32; BINARY_SZ_32],
) -> IntermediatePub<INTERMEDIATE_SZ_W_PADDING_32> {
    IntermediatePub(make_intermediate_array_32(binary))
}

#[cfg(feature = "dev-utils")]
pub fn make_intermediate_array_64_pub(
    binary: [u32; BINARY_SZ_64],
) -> IntermediatePub<INTERMEDIATE_SZ_W_PADDING_64> {
    IntermediatePub(make_intermediate_array_64(binary))
}

#[cfg(target_feature = "avx2")]
#[inline(always)]
fn intermediate_to_base58_32_avx(
    intermediate: &Intermediate<INTERMEDIATE_SZ_W_PADDING_32>,
    in_leading_0s: u64,
    out: &mut [u8],
) -> usize {
    let intermediate_ptr = intermediate.0.as_ptr() as *const i64;
    let intermediate0 = wl_ld(intermediate_ptr);
    let intermediate1 = wl_ld(unsafe { intermediate_ptr.offset(4) });
    let intermediate2 = wl_ld(unsafe { intermediate_ptr.offset(8) });
    let raw0 = intermediate_to_raw(intermediate0);
    let raw1 = intermediate_to_raw(intermediate1);
    let raw2 = intermediate_to_raw(intermediate2);
    let (compact0, compact1) = ten_per_slot_down_32(raw0, raw1, raw2);
    let raw_leading_0s = count_leading_zeros_45(compact0, compact1);
    let base58_0 = raw_to_base58(compact0);
    let base58_1 = raw_to_base58(compact1);
    let skip = raw_leading_0s as usize - in_leading_0s as usize;
    /* We know the final string is between 32 and 44 characters, so skip
     has to be in [1, 13].

     Suppose base58_0 is [ a0 a1 a2 ... af | b0 b1 b2 ... bf ] and skip
     is 2.
     It would be nice if we had something like the 128-bit barrel shifts
     we used in ten_per_slot_down, but they require immediates.
     Instead, we'll shift each ulong right by (skip%8) bytes:

     [ a2 a3 .. a7 0 0 aa ab .. af 0 0 | b2 b3 .. b7 0 0 ba .. bf 0 0 ]
     and maskstore to write just 8 bytes, skipping the first
     floor(skip/8) ulongs, to a starting address of out-8*floor(skip/8).

           out
             V
           [ a2 a3 a4 a5 a6 a7  0  0 -- -- ... ]

     Now we use another maskstore on the original base58_0, masking out
     the first floor(skip/8)+1 ulongs, and writing to out-skip:
           out
             V
     [ -- -- -- -- -- -- -- -- a8 a9 aa ab ... bd be bf ]

     Finally, we need to write the low 13 bytes from base58_1 and a '\0'
     terminator, starting at out-skip+32.  The easiest way to do this is
     actually to shift in 3 more bytes, write the full 16B and do it
     prior to the previous write:
                                               out-skip+29
                                                V
                                              [ 0  0  0  c0 c1 c2 .. cc ]
    */
    let w_skip = wl_bcast(skip as i64);
    let mod8_mask = wl_bcast(7);
    let compare = wl(0, 1, 2, 3);
    let shift_qty = wl_shl::<3>(wl_and(w_skip, mod8_mask)); /* bytes->bits */
    let shifted = wl_shru_vector(base58_0, shift_qty);
    let skip_div8 = wl_shru::<3>(w_skip);
    let mask1 = wl_eq(skip_div8, compare);
    let out_ptr = out.as_mut_ptr();
    let out_offset = unsafe { out_ptr.offset(-8 * (skip as isize / 8)) } as *mut i64;
    unsafe { _mm256_maskstore_epi64(out_offset, mask1, shifted) };
    let last = unsafe { _mm_bslli_si128(_mm256_extractf128_si256(base58_1, 0), 3) };
    unsafe { _mm_storeu_si128(out_ptr.offset(29 - skip as isize) as *mut __m128i, last) };
    let mask2 = wl_gt(compare, skip_div8);
    unsafe {
        _mm256_maskstore_epi64(
            out_ptr.offset(-(skip as isize)) as *mut i64,
            mask2,
            base58_0,
        )
    };
    skip
}

/// Encode a 32-byte array.
///
/// Mutates the provided `out` array and returns a u8 `len`
/// which indicates how many bytes of the `out` array were actually written to.
/// The remaining bytes are unchanged, hence the result we care about after
/// calling the function is `out[..len as usize]`.
///
/// # Examples
/// ```
/// let mut buf = [0u8; 44];
/// let bytes = &[
///     24, 243, 6, 223, 230, 153, 210, 8, 92, 137, 123, 67, 164, 197, 79, 196, 125, 43, 183,
///     85, 103, 91, 232, 167, 73, 131, 104, 131, 0, 101, 214, 231,
/// ];
/// let len = five8::encode_32(bytes, &mut buf);
/// assert_eq!(
///     &buf[..len as usize],
///     [
///         50, 103, 80, 105, 104, 85, 84, 106, 116, 51, 70, 74, 113, 102, 49, 86, 112, 105,
///         100, 103, 114, 89, 53, 99, 90, 54, 80, 117, 121, 77, 99, 99, 71, 86, 119, 81, 72,
///         82, 102, 106, 77, 80, 90, 71
///     ]
/// );
/// assert_eq!(len, 44);
/// ```
#[inline]
pub fn encode_32(bytes: &[u8; N_32], out: &mut [u8; BASE58_ENCODED_32_MAX_LEN]) -> u8 {
    let bytes_ptr = bytes as *const u8;
    let in_leading_0s = {
        #[cfg(target_feature = "avx2")]
        {
            in_leading_0s_32_avx(bytes_ptr)
        }
        #[cfg(not(target_feature = "avx2"))]
        {
            in_leading_0s_scalar::<N_32>(bytes_ptr)
        }
    };
    let binary = make_binary_array_32(bytes);
    let intermediate = make_intermediate_array_32(binary);
    let skip = intermediate_to_base58_32(&intermediate, in_leading_0s, out);
    RAW58_SZ_32 as u8 - skip as u8
}

#[inline(always)]
fn intermediate_to_base58_32(
    intermediate: &Intermediate<INTERMEDIATE_SZ_W_PADDING_32>,
    in_leading_0s: u64,
    out: &mut [u8],
) -> usize {
    #[cfg(not(target_feature = "avx2"))]
    {
        intermediate_to_base58_scalar::<INTERMEDIATE_SZ_W_PADDING_32, RAW58_SZ_32, INTERMEDIATE_SZ_32>(
            intermediate,
            in_leading_0s,
            out,
        )
    }
    #[cfg(target_feature = "avx2")]
    intermediate_to_base58_32_avx(intermediate, in_leading_0s, out)
}

#[inline(always)]
fn make_intermediate_array_32(
    binary: [u32; BINARY_SZ_32],
) -> Intermediate<INTERMEDIATE_SZ_W_PADDING_32> {
    /* Convert to the intermediate format:
      X = sum_i intermediate[i] * 58^(5*(INTERMEDIATE_SZ-1-i))
    Initially, we don't require intermediate[i] < 58^5, but we do want
    to make sure the sums don't overflow. */
    let mut intermediate = init_intermediate_array::<INTERMEDIATE_SZ_W_PADDING_32>();
    for i in 0..BINARY_SZ_32 {
        for j in 0..INTERMEDIATE_SZ_32 - 1 {
            let multiplier = unsafe { *ENC_TABLE_32.get_unchecked(i).get_unchecked(j) as u64 };
            add_binary_to_intermediate(&mut intermediate, j, binary, i, multiplier);
        }
    }
    adjust_intermediate_array::<INTERMEDIATE_SZ_W_PADDING_32, INTERMEDIATE_SZ_32>(
        &mut intermediate,
    );
    intermediate
}

#[cfg(test)]
mod tests {
    use crate::{decode_32, decode_64};
    #[cfg(target_feature = "avx2")]
    use core::array::from_fn;
    use five8_const::{decode_32_const, decode_64_const};
    use five8_core::{BASE58_ENCODED_32_MAX_LEN, BASE58_ENCODED_64_MAX_LEN};
    #[cfg(not(miri))]
    use prop::array::uniform32;
    #[cfg(not(miri))]
    use proptest::prelude::*;
    use std::string::String;

    use super::*;

    fn encode_32_to_string(bytes: &[u8; 32], buf: &mut [u8; BASE58_ENCODED_32_MAX_LEN]) -> String {
        let len = encode_32(bytes, buf);
        buf[..len as usize].iter().map(|c| *c as char).collect()
    }

    fn check_encode_decode_32(
        bytes: &[u8; 32],
        buf: &mut [u8; BASE58_ENCODED_32_MAX_LEN],
        encoded: &str,
    ) {
        assert_eq!(&encode_32_to_string(bytes, buf), encoded);
        let mut decoded = [0u8; 32];
        decode_32(encoded.as_bytes(), &mut decoded).unwrap();
        assert_eq!(&decoded, bytes);
        assert_eq!(&decode_32_const(encoded), bytes);
    }

    fn check_encode_decode_64(
        bytes: &[u8; 64],
        buf: &mut [u8; BASE58_ENCODED_64_MAX_LEN],
        encoded: &str,
    ) {
        assert_eq!(&encode_64_to_string(bytes, buf), encoded);
        let mut decoded = [0u8; 64];
        decode_64(encoded.as_bytes(), &mut decoded).unwrap();
        assert_eq!(&decoded, bytes);
        assert_eq!(&decode_64_const(encoded), bytes);
    }

    fn encode_64_to_string(bytes: &[u8; 64], buf: &mut [u8; BASE58_ENCODED_64_MAX_LEN]) -> String {
        let len = encode_64(bytes, buf);
        buf[..len as usize].iter().map(|c| *c as char).collect()
    }

    #[test]
    fn test_encode_decode_32() {
        let mut buf = [0u8; BASE58_ENCODED_32_MAX_LEN];
        let mut bytes = [0u8; 32];
        check_encode_decode_32(&bytes, &mut buf, "11111111111111111111111111111111");
        bytes[31] += 1;
        check_encode_decode_32(&bytes, &mut buf, "11111111111111111111111111111112");
        bytes[30] += 1;
        check_encode_decode_32(&bytes, &mut buf, "1111111111111111111111111111115S");
        let mut bytes = [255u8; 32];
        check_encode_decode_32(
            &bytes,
            &mut buf,
            "JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG",
        );
        bytes[31] -= 1;
        check_encode_decode_32(
            &bytes,
            &mut buf,
            "JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFF",
        );
        let bytes = [1u8; 32];
        check_encode_decode_32(
            &bytes,
            &mut buf,
            "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi",
        );
    }

    #[test]
    fn test_base58_decode_32() {
        let encoded = b"11111111111111111111111111111112";
        let encoded_ptr = encoded.as_ptr();
        assert_eq!(unsafe { *encoded_ptr.offset(31) }, b'2');
        let mut decoded = [0u8; 32];
        decode_32(encoded, &mut decoded).unwrap();
        let mut expected = [0u8; 32];
        expected[31] = 1;
        assert_eq!(expected, decoded);
    }

    #[test]
    fn test_encode_decode_64() {
        let mut buf = [0u8; BASE58_ENCODED_64_MAX_LEN];
        let mut bytes = [0u8; 64];
        check_encode_decode_64(
            &bytes,
            &mut buf,
            "1111111111111111111111111111111111111111111111111111111111111111",
        );
        bytes[63] += 1;
        check_encode_decode_64(
            &bytes,
            &mut buf,
            "1111111111111111111111111111111111111111111111111111111111111112",
        );
        bytes[62] += 1;
        check_encode_decode_64(
            &bytes,
            &mut buf,
            "111111111111111111111111111111111111111111111111111111111111115S",
        );
        let mut bytes = [255; 64];
        check_encode_decode_64(
            &bytes,
            &mut buf,
            "67rpwLCuS5DGA8KGZXKsVQ7dnPb9goRLoKfgGbLfQg9WoLUgNY77E2jT11fem3coV9nAkguBACzrU1iyZM4B8roQ",
        );
        bytes[63] -= 1;
        check_encode_decode_64(
            &bytes,
            &mut buf,
            "67rpwLCuS5DGA8KGZXKsVQ7dnPb9goRLoKfgGbLfQg9WoLUgNY77E2jT11fem3coV9nAkguBACzrU1iyZM4B8roP",
        );
    }

    #[cfg(target_feature = "avx2")]
    #[test]
    fn test_u8s_to_u32s_swapped_32() {
        let bytes: [u8; 32] = from_fn(|i| i as u8);
        let mut out = [0u8; 32];
        u8s_to_u32s_swapped_32(&bytes, &mut out);
        assert_eq!(
            out,
            [
                3, 2, 1, 0, 7, 6, 5, 4, 11, 10, 9, 8, 15, 14, 13, 12, 19, 18, 17, 16, 23, 22, 21,
                20, 27, 26, 25, 24, 31, 30, 29, 28
            ]
        );
    }

    #[cfg(target_feature = "avx2")]
    #[test]
    fn test_u8s_to_u32s_swapped_64() {
        let bytes: [u8; 64] = from_fn(|i| i as u8);
        let mut out = [0u8; N_64];
        u8s_to_u32s_swapped_64(&bytes, &mut out);
        assert_eq!(
            out,
            [
                3, 2, 1, 0, 7, 6, 5, 4, 11, 10, 9, 8, 15, 14, 13, 12, 19, 18, 17, 16, 23, 22, 21,
                20, 27, 26, 25, 24, 31, 30, 29, 28, 35, 34, 33, 32, 39, 38, 37, 36, 43, 42, 41, 40,
                47, 46, 45, 44, 51, 50, 49, 48, 55, 54, 53, 52, 59, 58, 57, 56, 63, 62, 61, 60
            ]
        );
    }

    #[cfg(not(miri))]
    proptest! {
        #[test]
        fn proptest_encode_32(key in uniform32(0u8..)) {
            let bs58_res = bs58::encode(key).into_vec();
            let mut out = [0u8; BASE58_ENCODED_32_MAX_LEN];
            let len = encode_32(&key, &mut out);
            assert_eq!(bs58_res, out[..len as usize].to_vec());
        }
    }

    #[cfg(not(miri))]
    proptest! {
        #[test]
        fn proptest_encode_64(first_half in uniform32(0u8..), second_half in uniform32(0u8..)) {
            let mut combined = [0u8; 64];
            combined[..32].copy_from_slice(&first_half);
            combined[32..].copy_from_slice(&second_half);
            let bs58_res = bs58::encode(combined).into_vec();
            let mut out = [0u8; BASE58_ENCODED_64_MAX_LEN];
            let len = encode_64(&combined, &mut out);
            assert_eq!(bs58_res, out[..len as usize].to_vec());
        }
    }
}
