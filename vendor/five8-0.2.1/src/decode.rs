#![allow(clippy::missing_transmute_annotations)]
#[cfg(target_feature = "avx2")]
use core::mem::transmute;

use core::array::from_fn;

use five8_core::{
    DecodeError, BASE58_ENCODED_32_MAX_LEN, BASE58_ENCODED_64_MAX_LEN, BASE58_INVALID_CHAR,
    BASE58_INVERSE, BASE58_INVERSE_TABLE_OFFSET, BASE58_INVERSE_TABLE_SENTINEL, BINARY_SZ_32,
    BINARY_SZ_64, DEC_TABLE_32, DEC_TABLE_64, INTERMEDIATE_SZ_32, INTERMEDIATE_SZ_64, N_32, N_64,
    RAW58_SZ_32, RAW58_SZ_64,
};

use crate::unlikely::unlikely;

#[cfg(feature = "dev-utils")]
pub fn truncate_and_swap_u64s_scalar_pub<const BINARY_SZ: usize, const N: usize>(
    out: &mut [u8; N],
    binary: &[u64; BINARY_SZ],
) {
    truncate_and_swap_u64s_scalar(out, binary);
}

#[cfg(any(not(target_feature = "avx2"), feature = "dev-utils"))]
#[inline(always)]
fn truncate_and_swap_u64s_scalar<const BINARY_SZ: usize, const N: usize>(
    out: &mut [u8; N],
    binary: &[u64; BINARY_SZ],
) {
    let binary_u8 = binary.as_ptr() as *const u8;
    for i in 0..BINARY_SZ {
        // take the first four bytes of each 8-byte block and reverse them:
        // 3 2 1 0 11 10 9 8 19 18 17 16 27 26 25 24 etc
        // or if on a BE machine, just take the last four bytes of each 8-byte block:
        // 4 5 6 7 12 13 14 15 20 21 22 23 etc
        let binary_u8_idx = i * 8;
        let out_idx = i * 4;
        #[cfg(target_endian = "little")]
        unsafe {
            *out.get_unchecked_mut(out_idx) = *binary_u8.add(binary_u8_idx + 3);
            *out.get_unchecked_mut(out_idx + 1) = *binary_u8.add(binary_u8_idx + 2);
            *out.get_unchecked_mut(out_idx + 2) = *binary_u8.add(binary_u8_idx + 1);
            *out.get_unchecked_mut(out_idx + 3) = *binary_u8.add(binary_u8_idx);
        }
        #[cfg(target_endian = "big")]
        unsafe {
            *out.get_unchecked_mut(out_idx) = *binary_u8.add(binary_u8_idx + 4);
            *out.get_unchecked_mut(out_idx + 1) = *binary_u8.add(binary_u8_idx + 5);
            *out.get_unchecked_mut(out_idx + 2) = *binary_u8.add(binary_u8_idx + 6);
            *out.get_unchecked_mut(out_idx + 3) = *binary_u8.add(binary_u8_idx + 7);
        }
    }
}

#[inline(always)]
fn base58_decode_after_be_convert<const N: usize>(
    out: &[u8; N],
    encoded: &[u8],
) -> Result<(), DecodeError> {
    /* Make sure the encoded version has the same number of leading '1's
    as the decoded version has leading 0s. The check doesn't read past
    the end of encoded, because '\0' != '1', so it will return NULL. */
    let mut leading_zero_cnt = 0u64;
    while leading_zero_cnt < N as u64 {
        if unlikely(leading_zero_cnt as usize >= encoded.len()) {
            return Err(DecodeError::TooShort);
        }
        let out_val = unsafe { *out.get_unchecked(leading_zero_cnt as usize) };
        if out_val != 0 {
            break;
        }
        if unlikely(unsafe { *encoded.get_unchecked(leading_zero_cnt as usize) != b'1' }) {
            return Err(DecodeError::TooShort);
        }
        leading_zero_cnt += 1;
    }
    if unlikely(
        encoded
            .get(leading_zero_cnt as usize)
            .map_or(false, |x| *x == b'1'),
    ) {
        return Err(DecodeError::OutputTooLong);
    }
    Ok(())
}

#[inline(always)]
fn base58_decode_before_be_convert<
    const ENCODED_LEN: usize,
    const RAW58_SZ: usize,
    const INTERMEDIATE_SZ: usize,
    const BINARY_SZ: usize,
>(
    encoded: &[u8],
    dec_table: &[[u32; BINARY_SZ]; INTERMEDIATE_SZ],
) -> Result<[u64; BINARY_SZ], DecodeError> {
    let mut char_cnt = 0usize;
    while char_cnt < (ENCODED_LEN + 1).min(encoded.len()) {
        let c = encoded[char_cnt];
        /* If c<'1', this will underflow and idx will be huge */
        let idx = (c as u64).wrapping_sub(BASE58_INVERSE_TABLE_OFFSET as u64);
        let idx = idx.min(BASE58_INVERSE_TABLE_SENTINEL as u64);
        char_cnt += 1;
        if unlikely(unsafe { *BASE58_INVERSE.get_unchecked(idx as usize) } == BASE58_INVALID_CHAR) {
            return Err(DecodeError::InvalidChar(c));
        }
    }
    if unlikely(char_cnt == ENCODED_LEN + 1) {
        /* too long */
        return Err(DecodeError::TooLong);
    }
    let prepend_0 = RAW58_SZ - char_cnt;
    let raw_base58: [u8; RAW58_SZ] = from_fn(|j| {
        if j < prepend_0 {
            0
        } else {
            unsafe {
                *BASE58_INVERSE.get_unchecked(
                    (*encoded.get_unchecked(j - prepend_0) - BASE58_INVERSE_TABLE_OFFSET) as usize,
                )
            }
        }
    });
    let intermediate: [u64; INTERMEDIATE_SZ] = from_fn(|i| unsafe {
        *raw_base58.get_unchecked(5 * i) as u64 * 11316496
            + *raw_base58.get_unchecked(5 * i + 1) as u64 * 195112
            + *raw_base58.get_unchecked(5 * i + 2) as u64 * 3364
            + *raw_base58.get_unchecked(5 * i + 3) as u64 * 58
            + *raw_base58.get_unchecked(5 * i + 4) as u64
    });
    let mut binary: [u64; BINARY_SZ] = from_fn(|j| {
        let mut acc = 0u64;
        for i in 0..INTERMEDIATE_SZ {
            acc += unsafe {
                intermediate.get_unchecked(i) * *dec_table.get_unchecked(i).get_unchecked(j) as u64
            };
        }
        acc
    });
    for i in (1..BINARY_SZ).rev() {
        unsafe {
            *binary.get_unchecked_mut(i - 1) += binary.get_unchecked(i) >> 32;
        }
        unsafe {
            *binary.get_unchecked_mut(i) &= 0xFFFFFFFF;
        }
    }
    if unlikely(unsafe { *binary.get_unchecked(0) } > 0xFFFFFFFF) {
        return Err(DecodeError::LargestTermTooHigh);
    }
    Ok(binary)
}

/// Decode base58 data onto a 32-byte array.
///
/// # Examples
///
/// ```
/// let bytes = b"2gPihUTjt3FJqf1VpidgrY5cZ6PuyMccGVwQHRfjMPZG";
/// let mut out = [0u8; 32];
/// five8::decode_32(bytes, &mut out).unwrap();
/// assert_eq!(
///     out,
///     [
///         24, 243, 6, 223, 230, 153, 210, 8, 92, 137, 123, 67, 164, 197, 79, 196, 125, 43,
///         183, 85, 103, 91, 232, 167, 73, 131, 104, 131, 0, 101, 214, 231
///     ]
/// );
/// ```
#[inline]
pub fn decode_32<I: AsRef<[u8]>>(encoded: I, out: &mut [u8; N_32]) -> Result<(), DecodeError> {
    let as_ref = encoded.as_ref();
    let binary = base58_decode_before_be_convert::<
        BASE58_ENCODED_32_MAX_LEN,
        RAW58_SZ_32,
        INTERMEDIATE_SZ_32,
        BINARY_SZ_32,
    >(as_ref, &DEC_TABLE_32)?;
    /* Convert each term to big endian for the final output */
    #[cfg(target_feature = "avx2")]
    truncate_and_swap_u64s_32(out, &binary);
    #[cfg(not(target_feature = "avx2"))]
    truncate_and_swap_u64s_scalar(out, &binary);
    base58_decode_after_be_convert(out, as_ref)
}

/// Decode base58 data onto a 64-byte array.
///
/// # Examples
///
/// ```
/// let bytes = b"11cgTH4D5e8S3snD444WbbGrkepjTvWMj2jkmCGJtgn3H7qrPb1BnwapxpbGdRtHQh9t9Wbn9t6ZDGHzWpL4df";
/// let mut out = [0u8; 64];
/// five8::decode_64(bytes, &mut out).unwrap();
/// assert_eq!(
///     out,
///     [
///         0, 0, 10, 85, 198, 191, 71, 18, 5, 54, 6, 255, 181, 32, 227, 150, 208, 3, 157, 135, 222, 67, 50,
///         23, 237, 51, 240, 123, 34, 148, 111, 84, 98, 162, 236, 133, 31, 93, 185, 142, 108, 41, 191, 1, 138,
///         6, 192, 0, 46, 93, 25, 65, 243, 223, 225, 225, 85, 55, 82, 251, 109, 132, 165, 2
///     ]
/// );
/// ```
#[inline]
pub fn decode_64<I: AsRef<[u8]>>(encoded: I, out: &mut [u8; N_64]) -> Result<(), DecodeError> {
    let as_ref = encoded.as_ref();
    let binary = base58_decode_before_be_convert::<
        BASE58_ENCODED_64_MAX_LEN,
        RAW58_SZ_64,
        INTERMEDIATE_SZ_64,
        BINARY_SZ_64,
    >(as_ref, &DEC_TABLE_64)?;
    /* Convert each term to big endian for the final output */
    #[cfg(target_feature = "avx2")]
    truncate_and_swap_u64s_64(out, &binary);
    #[cfg(not(target_feature = "avx2"))]
    truncate_and_swap_u64s_scalar(out, &binary);
    base58_decode_after_be_convert(out, as_ref)
}

#[cfg(target_feature = "avx2")]
#[inline(always)]
fn truncate_and_swap_u64s_32(out: &mut [u8; N_32], nums: &[u64; BINARY_SZ_32]) {
    let res = truncate_and_swap_u64s_registers::<BINARY_SZ_32, N_32, 2>(nums);
    *out = unsafe { transmute(res) }
}

#[cfg(feature = "dev-utils")]
pub fn truncate_and_swap_u64s_64_pub(out: &mut [u8; N_64], nums: &[u64; BINARY_SZ_64]) {
    truncate_and_swap_u64s_64(out, nums)
}

#[cfg(any(target_feature = "avx2", feature = "dev-utils"))]
#[inline(always)]
fn truncate_and_swap_u64s_64(out: &mut [u8; N_64], nums: &[u64; BINARY_SZ_64]) {
    let res = truncate_and_swap_u64s_registers::<BINARY_SZ_64, N_64, 4>(nums);
    *out = unsafe { core::mem::transmute(res) }
}

// unclear if this helps performance
#[cfg(any(target_feature = "avx2", feature = "dev-utils"))]
#[inline(always)]
fn truncate_and_swap_u64s_registers<
    const BINARY_SZ: usize,
    const N: usize,
    const N_REGISTERS: usize,
>(
    nums: &[u64; BINARY_SZ],
) -> [core::arch::x86_64::__m128i; N_REGISTERS] {
    let mask = unsafe {
        core::arch::x86_64::_mm256_set_epi8(
            -128, -128, -128, -128, -128, -128, -128, -128, 8, 9, 10, 11, 0, 1, 2, 3, -128, -128,
            -128, -128, -128, -128, -128, -128, 8, 9, 10, 11, 0, 1, 2, 3,
        )
    };
    let mut holder: [core::arch::x86_64::__m256i; N_REGISTERS] = from_fn(|i| unsafe {
        core::arch::x86_64::_mm256_loadu_si256(
            nums.get_unchecked((i * 4)..(4 + i * 4)).as_ptr() as *const core::arch::x86_64::__m256i
        )
    });
    for i in 0..N_REGISTERS {
        let register = unsafe { *holder.get_unchecked(i) };
        unsafe {
            *holder.get_unchecked_mut(i) = core::arch::x86_64::_mm256_shuffle_epi8(register, mask)
        };
    }
    let splits: [[core::arch::x86_64::__m128i; 2]; N_REGISTERS] =
        from_fn(|i| unsafe { core::mem::transmute(*holder.get_unchecked(i)) });
    from_fn(|i| {
        let split = unsafe { *splits.get_unchecked(i) };
        unsafe {
            core::arch::x86_64::_mm_unpacklo_epi64(*split.get_unchecked(0), *split.get_unchecked(1))
        }
    })
}

#[cfg(test)]
mod tests {
    #[cfg(target_feature = "avx2")]
    use core::arch::x86_64::{_mm256_shuffle_epi32, _mm256_unpacklo_epi64};
    #[cfg(not(miri))]
    use prop::array::uniform32;
    #[cfg(not(miri))]
    use proptest::prelude::*;

    use super::*;

    fn check_bad_decode_32(expected_err: DecodeError, encoded: &str) {
        let mut decoded = [0u8; 32];
        let err = decode_32(encoded.as_bytes(), &mut decoded).unwrap_err();
        assert_eq!(err, expected_err);
    }

    fn check_bad_decode_64(expected_err: DecodeError, encoded: &str) {
        let mut decoded = [0u8; 64];
        let err = decode_64(encoded.as_bytes(), &mut decoded).unwrap_err();
        assert_eq!(err, expected_err);
    }

    #[test]
    fn test_decode_error_32() {
        check_bad_decode_32(DecodeError::TooShort, "1");
        check_bad_decode_32(DecodeError::TooShort, "1111111111111111111111111111111");
        check_bad_decode_32(
            DecodeError::TooShort,
            "4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJz",
        );
        check_bad_decode_32(
            DecodeError::TooShort,
            "4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofL",
        );
        check_bad_decode_32(
            DecodeError::TooLong,
            "4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofLRda4",
        );
        check_bad_decode_32(
            DecodeError::OutputTooLong,
            "111111111111111111111111111111111",
        );
        check_bad_decode_32(
            DecodeError::LargestTermTooHigh,
            "JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFJ",
        ); /* 2nd-smallest 33 byte value that doesn't start with 0x0 */
        check_bad_decode_32(
            DecodeError::OutputTooLong,
            "11aEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWx",
        );
        check_bad_decode_32(
            DecodeError::InvalidChar(48),
            "11111111111111111111111111111110",
        );
        check_bad_decode_32(
            DecodeError::InvalidChar(33),
            "1111111111111111111111111111111!",
        );
        check_bad_decode_32(
            DecodeError::InvalidChar(73),
            "1111111111111111111111111111111I",
        );
        check_bad_decode_32(
            DecodeError::InvalidChar(79),
            "1111111111111111111111111111111O",
        );
        check_bad_decode_32(
            DecodeError::InvalidChar(95),
            "1111111111111111111111111111111_",
        );
        check_bad_decode_32(
            DecodeError::InvalidChar(108),
            "1111111111111111111111111111111l",
        );
    }

    #[test]
    fn test_decode_unprintable_32() {
        let encoded = [
            49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49,
            49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 0, 1, 0, 0, 0, 0, 0, 127,
        ];
        let mut out = [0u8; 32];
        let err = decode_32(encoded, &mut out).unwrap_err();
        assert_eq!(err, DecodeError::InvalidChar(0));
    }

    #[test]
    fn test_decode_error_64() {
        check_bad_decode_64(DecodeError::TooShort, "1");
        check_bad_decode_64(
            DecodeError::TooShort,
            "111111111111111111111111111111111111111111111111111111111111111",
        );
        check_bad_decode_64(
            DecodeError::TooShort,
            "2AFv15MNPuA84RmU66xw2uMzGipcVxNpzAffoacGVvjFue3CBmf633fAWuiP9cwL9C3z3CJiGgRSFjJfeEcA",
        );
        check_bad_decode_64(DecodeError::TooShort, "2AFv15MNPuA84RmU66xw2uMzGipcVxNpzAffoacGVvjFue3CBmf633fAWuiP9cwL9C3z3CJiGgRSFjJfeEcA6QW");
        check_bad_decode_64(DecodeError::TooLong, "2AFv15MNPuA84RmU66xw2uMzGipcVxNpzAffoacGVvjFue3CBmf633fAWuiP9cwL9C3z3CJiGgRSFjJfeEcA6QWabc");
        check_bad_decode_64(
            DecodeError::OutputTooLong,
            "11111111111111111111111111111111111111111111111111111111111111111",
        );
        check_bad_decode_64(
            DecodeError::LargestTermTooHigh,
            "67rpwLCuS5DGA8KGZXKsVQ7dnPb9goRLoKfgGbLfQg9WoLUgNY77E2jT11fem3coV9nAkguBACzrU1iyZM4B8roS"
        ); /* 2nd-smallest 65 byte value that doesn't start with 0x0 */

        check_bad_decode_64(DecodeError::OutputTooLong, "1114tjGcyzrfXw2deDmDAFFaFyss32WRgkYdDJuprrNEL8kc799TrHSQHfE9fv6ZDBUg2dsMJdfYr71hjE4EfjEN"); /* Start with too many '1's */
        check_bad_decode_64(
            DecodeError::InvalidChar(48),
            "1111111111111111111111111111111111111111111111111111111111111110",
        );
        check_bad_decode_64(
            DecodeError::InvalidChar(33),
            "111111111111111111111111111111111111111111111111111111111111111!",
        );
        check_bad_decode_64(
            DecodeError::InvalidChar(59),
            "111111111111111111111111111111111111111111111111111111111111111;",
        );
        check_bad_decode_64(
            DecodeError::InvalidChar(73),
            "111111111111111111111111111111111111111111111111111111111111111I;",
        );
        check_bad_decode_64(
            DecodeError::InvalidChar(79),
            "111111111111111111111111111111111111111111111111111111111111111O",
        );
        check_bad_decode_64(
            DecodeError::InvalidChar(95),
            "111111111111111111111111111111111111111111111111111111111111111_",
        );
        check_bad_decode_64(
            DecodeError::InvalidChar(108),
            "111111111111111111111111111111111111111111111111111111111111111l",
        );
    }

    #[cfg(target_feature = "avx2")]
    #[test]
    fn test_pshufb_32() {
        // take the first four bytes of each 8-byte block and reverse them:
        // 3 2 1 0 11 10 9 8 19 18 17 16 27 26 25 24 etc

        let bytes: [u8; 64] = from_fn(|i| i as u8);
        let nums = unsafe { transmute(bytes) };
        let mut out = [0u8; 32];
        truncate_and_swap_u64s_32(&mut out, &nums);
        assert_eq!(
            out,
            [
                3, 2, 1, 0, 11, 10, 9, 8, 19, 18, 17, 16, 27, 26, 25, 24, 35, 34, 33, 32, 43, 42,
                41, 40, 51, 50, 49, 48, 59, 58, 57, 56
            ]
        );
    }

    #[cfg(target_feature = "avx2")]
    #[test]
    fn test_pshufb_64() {
        // take the first four bytes of each 8-byte block and reverse them:
        // 3 2 1 0 11 10 9 8 19 18 17 16 27 26 25 24 etc

        let bytes: [u8; 128] = from_fn(|i| i as u8);
        let nums = unsafe { transmute(bytes) };
        let mut out = [0u8; 64];
        truncate_and_swap_u64s_64(&mut out, &nums);
        assert_eq!(
            out,
            [
                3, 2, 1, 0, 11, 10, 9, 8, 19, 18, 17, 16, 27, 26, 25, 24, 35, 34, 33, 32, 43, 42,
                41, 40, 51, 50, 49, 48, 59, 58, 57, 56, 67, 66, 65, 64, 75, 74, 73, 72, 83, 82, 81,
                80, 91, 90, 89, 88, 99, 98, 97, 96, 107, 106, 105, 104, 115, 114, 113, 112, 123,
                122, 121, 120
            ]
        );
    }

    #[cfg(target_feature = "avx2")]
    #[test]
    fn test_pshufb_tmp() {
        let bytes: [u32; 8] = from_fn(|i| i as u32);
        let bytes2: [u32; 8] = from_fn(|i| i as u32 + 8);
        let res = unsafe { _mm256_shuffle_epi32::<0b00_00_10_00>(core::mem::transmute(bytes)) };
        let res2 = unsafe { _mm256_shuffle_epi32::<0b00_00_10_00>(core::mem::transmute(bytes2)) };
        let out: [u32; 8] = unsafe { core::mem::transmute(res) };
        let out2: [u32; 8] = unsafe { core::mem::transmute(res2) };
        std::println!("out: {out:?}");
        std::println!("out2: {out2:?}");
        let unpacked = unsafe { _mm256_unpacklo_epi64(res, res2) };
        let out3: [u32; 8] = unsafe { core::mem::transmute(unpacked) };
        std::println!("out3: {out3:?}");
    }

    #[cfg(not(miri))]
    proptest! {
        #[test]
        fn proptest_decode_32(key in uniform32(0u8..)) {
            let encoded = bs58::encode(key).into_vec();
            let bs58_res = bs58::decode(&encoded).into_vec().unwrap();
            let const_res = five8_const::decode_32_const(&std::string::String::from_utf8(encoded.clone()).unwrap());
            let mut out = [0u8; 32];
            decode_32(&encoded, &mut out).unwrap();
            assert_eq!(bs58_res, out.to_vec());
            assert_eq!(const_res, out);
        }
    }

    #[cfg(not(miri))]
    proptest! {
        #[test]
        fn proptest_decode_64(first_half in uniform32(0u8..), second_half in uniform32(0u8..)) {
            let mut combined = [0u8; 64];
            combined[..32].copy_from_slice(&first_half);
            combined[32..].copy_from_slice(&second_half);
            let encoded = bs58::encode(combined).into_vec();
            let bs58_res = bs58::decode(&encoded).into_vec().unwrap();
            let const_res = five8_const::decode_64_const(&std::string::String::from_utf8(encoded.clone()).unwrap());
            let mut out = [0u8; 64];
            decode_64(&encoded, &mut out).unwrap();
            assert_eq!(bs58_res, out.to_vec());
            assert_eq!(const_res, out);
        }
    }
}
