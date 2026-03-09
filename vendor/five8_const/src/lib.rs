#![doc = include_str!("../README.md")]
#![no_std]
use five8_core::{
    DecodeError, BASE58_ENCODED_32_MAX_LEN, BASE58_ENCODED_64_MAX_LEN, BASE58_INVALID_CHAR,
    BASE58_INVERSE, BASE58_INVERSE_TABLE_OFFSET, BASE58_INVERSE_TABLE_SENTINEL, BINARY_SZ_32,
    BINARY_SZ_64, DEC_TABLE_32, DEC_TABLE_64, INTERMEDIATE_SZ_32, INTERMEDIATE_SZ_64, N_32, N_64,
    RAW58_SZ_32, RAW58_SZ_64,
};

const fn unwrap_const(err: DecodeError) -> ! {
    match err {
        DecodeError::InvalidChar(_) => panic!("Illegal base58 char"),
        DecodeError::TooLong => panic!("Base58 string too long"),
        DecodeError::TooShort => panic!("Base58 string too short"),
        DecodeError::LargestTermTooHigh => panic!("Largest term greater than 2^32"),
        DecodeError::OutputTooLong => panic!("Decoded output has too many bytes"),
    }
}

const fn base58_decode_before_be_convert_const<
    const ENCODED_LEN: usize,
    const RAW58_SZ: usize,
    const INTERMEDIATE_SZ: usize,
    const BINARY_SZ: usize,
>(
    encoded: &[u8],
    dec_table: &[[u32; BINARY_SZ]; INTERMEDIATE_SZ],
) -> Result<[u64; BINARY_SZ], DecodeError> {
    let mut char_cnt = 0usize;
    let min_left = ENCODED_LEN + 1;
    let min_right = encoded.len();
    let num_iters = if min_left < min_right {
        min_left
    } else {
        min_right
    };
    while char_cnt < num_iters {
        let c = encoded[char_cnt];
        /* If c<'1', this will underflow and idx will be huge */
        let idx = (c as u64).wrapping_sub(BASE58_INVERSE_TABLE_OFFSET as u64);
        let capped_idx = if idx < BASE58_INVERSE_TABLE_SENTINEL as u64 {
            idx
        } else {
            BASE58_INVERSE_TABLE_SENTINEL as u64
        };
        char_cnt += 1;
        if BASE58_INVERSE[capped_idx as usize] == BASE58_INVALID_CHAR {
            return Err(DecodeError::InvalidChar(c));
        }
    }
    if char_cnt == ENCODED_LEN + 1 {
        /* too long */
        return Err(DecodeError::TooLong);
    }
    let prepend_0 = RAW58_SZ - char_cnt;
    let mut raw_base58 = [0u8; RAW58_SZ];
    let mut j = 0;
    while j < RAW58_SZ {
        raw_base58[j] = if j < prepend_0 {
            0
        } else {
            BASE58_INVERSE[(encoded[j - prepend_0] - BASE58_INVERSE_TABLE_OFFSET) as usize]
        };
        j += 1;
    }
    let mut intermediate = [0u64; INTERMEDIATE_SZ];
    let mut i = 0;
    while i < INTERMEDIATE_SZ {
        intermediate[i] = raw_base58[5 * i] as u64 * 11316496
            + raw_base58[5 * i + 1] as u64 * 195112
            + raw_base58[5 * i + 2] as u64 * 3364
            + raw_base58[5 * i + 3] as u64 * 58
            + raw_base58[5 * i + 4] as u64;
        i += 1;
    }
    let mut binary = [0u64; BINARY_SZ];
    let mut k = 0;
    while k < BINARY_SZ {
        let mut acc = 0u64;
        let mut l = 0;
        while l < INTERMEDIATE_SZ {
            acc += intermediate[l] * dec_table[l][k] as u64;
            l += 1;
        }
        binary[k] = acc;
        k += 1;
    }
    let mut m = BINARY_SZ - 1;
    while m >= 1 {
        binary[m - 1] += binary[m] >> 32;
        binary[m] &= 0xFFFFFFFF;
        m -= 1;
    }
    if binary[0] > 0xFFFFFFFF {
        return Err(DecodeError::LargestTermTooHigh);
    }
    Ok(binary)
}

const fn base58_decode_after_be_convert_const<const N: usize>(
    out: &[u8; N],
    encoded: &[u8],
) -> Result<(), DecodeError> {
    /* Make sure the encoded version has the same number of leading '1's
    as the decoded version has leading 0s. The check doesn't read past
    the end of encoded, because '\0' != '1', so it will return NULL. */
    let mut leading_zero_cnt = 0u64;
    while leading_zero_cnt < N as u64 {
        if leading_zero_cnt as usize >= encoded.len() {
            return Err(DecodeError::TooShort);
        }
        let out_val = out[leading_zero_cnt as usize];
        if out_val != 0 {
            break;
        }
        if encoded[leading_zero_cnt as usize] != b'1' {
            return Err(DecodeError::TooShort);
        }
        leading_zero_cnt += 1;
    }
    if leading_zero_cnt as usize > N {
        return Err(DecodeError::OutputTooLong);
    }
    if (leading_zero_cnt as usize) < N && encoded[leading_zero_cnt as usize] == b'1' {
        return Err(DecodeError::OutputTooLong);
    }
    Ok(())
}

const fn truncate_and_swap_u64s_const<const BINARY_SZ: usize, const N: usize>(
    binary: &[u64; BINARY_SZ],
) -> [u8; N] {
    let mut out = [0u8; N];
    let binary_u8 = binary.as_ptr() as *const u8;
    let mut i = 0;
    while i < BINARY_SZ {
        // take the first four bytes of each 8-byte block and reverse them:
        // 3 2 1 0 11 10 9 8 19 18 17 16 27 26 25 24 etc
        // or if on a BE machine, just take the last four bytes of each 8-byte block:
        // 4 5 6 7 12 13 14 15 20 21 22 23 etc
        let binary_u8_idx = i * 8;
        let out_idx = i * 4;
        #[cfg(target_endian = "little")]
        unsafe {
            out[out_idx] = *binary_u8.add(binary_u8_idx + 3);
            out[out_idx + 1] = *binary_u8.add(binary_u8_idx + 2);
            out[out_idx + 2] = *binary_u8.add(binary_u8_idx + 1);
            out[out_idx + 3] = *binary_u8.add(binary_u8_idx);
        }
        #[cfg(target_endian = "big")]
        unsafe {
            out[out_idx] = *binary_u8.add(binary_u8_idx + 4);
            out[out_idx + 1] = *binary_u8.add(binary_u8_idx + 5);
            out[out_idx + 2] = *binary_u8.add(binary_u8_idx + 6);
            out[out_idx + 3] = *binary_u8.add(binary_u8_idx + 7);
        }
        i += 1
    }
    out
}

const fn decode_const<
    const N: usize,
    const ENCODED_LEN: usize,
    const RAW58_SZ: usize,
    const INTERMEDIATE_SZ: usize,
    const BINARY_SZ: usize,
>(
    encoded: &str,
    dec_table: &[[u32; BINARY_SZ]; INTERMEDIATE_SZ],
) -> Result<[u8; N], DecodeError> {
    let as_ref = encoded.as_bytes();
    let binary_res =
        base58_decode_before_be_convert_const::<ENCODED_LEN, RAW58_SZ, INTERMEDIATE_SZ, BINARY_SZ>(
            as_ref, dec_table,
        );
    let binary = match binary_res {
        Ok(x) => x,
        Err(e) => return Err(e),
    };
    /* Convert each term to big endian for the final output */
    let out: [u8; N] = truncate_and_swap_u64s_const(&binary);
    match base58_decode_after_be_convert_const(&out, as_ref) {
        Ok(()) => Ok(out),
        Err(e) => Err(e),
    }
}

const fn decode_const_unwrap<
    const N: usize,
    const ENCODED_LEN: usize,
    const RAW58_SZ: usize,
    const INTERMEDIATE_SZ: usize,
    const BINARY_SZ: usize,
>(
    encoded: &str,
    dec_table: &[[u32; BINARY_SZ]; INTERMEDIATE_SZ],
) -> [u8; N] {
    match decode_const::<N, ENCODED_LEN, RAW58_SZ, INTERMEDIATE_SZ, BINARY_SZ>(encoded, dec_table) {
        Ok(x) => x,
        Err(e) => unwrap_const(e),
    }
}

/// Try decode into a 32-byte array.
pub const fn try_decode_32_const(encoded: &str) -> Result<[u8; N_32], DecodeError> {
    decode_const::<N_32, BASE58_ENCODED_32_MAX_LEN, RAW58_SZ_32, INTERMEDIATE_SZ_32, BINARY_SZ_32>(
        encoded,
        &DEC_TABLE_32,
    )
}

/// Decode into a 32-byte array. Panic on error.
pub const fn decode_32_const(encoded: &str) -> [u8; N_32] {
    decode_const_unwrap::<
        N_32,
        BASE58_ENCODED_32_MAX_LEN,
        RAW58_SZ_32,
        INTERMEDIATE_SZ_32,
        BINARY_SZ_32,
    >(encoded, &DEC_TABLE_32)
}

/// Try decode into a 64-byte array.
pub const fn try_decode_64_const(encoded: &str) -> Result<[u8; N_64], DecodeError> {
    decode_const::<N_64, BASE58_ENCODED_64_MAX_LEN, RAW58_SZ_64, INTERMEDIATE_SZ_64, BINARY_SZ_64>(
        encoded,
        &DEC_TABLE_64,
    )
}

/// Decode into a 64-byte array. Panic on error.
pub const fn decode_64_const(encoded: &str) -> [u8; N_64] {
    decode_const_unwrap::<
        N_64,
        BASE58_ENCODED_64_MAX_LEN,
        RAW58_SZ_64,
        INTERMEDIATE_SZ_64,
        BINARY_SZ_64,
    >(encoded, &DEC_TABLE_64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECODE_32_CONST_EXAMPLE: [u8; N_32] =
        decode_32_const("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFF");

    #[test]
    fn test_decode_const_ok() {
        let mut expected = [255u8; 32];
        expected[31] = 254;
        assert_eq!(DECODE_32_CONST_EXAMPLE, expected);
    }

    #[test]
    #[should_panic]
    fn test_decode_const_small_buffer_panic() {
        decode_32_const("a3gV");
    }

    #[test]
    #[should_panic]
    fn test_decode_const_invalid_char_panic() {
        let sample = "123456789abcd!efghij";
        decode_32_const(sample);
    }
}
