#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![no_std]
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
use core::fmt;
pub const BASE58_INVERSE_TABLE_OFFSET: u8 = b'1';
pub const BASE58_INVERSE_TABLE_SENTINEL: u8 = 1 + b'z' - BASE58_INVERSE_TABLE_OFFSET;

pub const BASE58_INVALID_CHAR: u8 = 255;
pub const BASE58_ENCODED_32_MAX_LEN: usize = 44; /* Computed as ceil(log_58(256^32 - 1)) */
pub const BASE58_ENCODED_64_MAX_LEN: usize = 88; /* Computed as ceil(log_58(256^64 - 1)) */
pub const INTERMEDIATE_SZ_32: usize = 9; /* Computed by ceil(log_(58^5) (256^32-1)) */
pub const INTERMEDIATE_SZ_64: usize = 18; /* Computed by ceil(log_(58^5) (256^64-1)) */
pub const RAW58_SZ_32: usize = INTERMEDIATE_SZ_32 * 5;
pub const RAW58_SZ_64: usize = INTERMEDIATE_SZ_64 * 5;
pub const N_32: usize = 32;
pub const N_64: usize = 64;
pub const BINARY_SZ_32: usize = N_32 / 4;
pub const BINARY_SZ_64: usize = N_64 / 4;

const BAD: u8 = BASE58_INVALID_CHAR;
pub const BASE58_INVERSE: [u8; 75] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, BAD, BAD, BAD, BAD, BAD, BAD, BAD, 9, 10, 11, 12, 13, 14, 15, 16,
    BAD, 17, 18, 19, 20, 21, BAD, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, BAD, BAD, BAD, BAD,
    BAD, BAD, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, BAD, 44, 45, 46, 47, 48, 49, 50, 51, 52,
    53, 54, 55, 56, 57, BAD,
];

/* Contains the unique values less than 2^32 such that:
58^(5*(8-j)) = sum_k table[j][k]*2^(32*(7-k)) */
pub const DEC_TABLE_32: [[u32; BINARY_SZ_32]; INTERMEDIATE_SZ_32] = [
    [
        1277, 2650397687, 3801011509, 2074386530, 3248244966, 687255411, 2959155456, 0,
    ],
    [
        0, 8360, 1184754854, 3047609191, 3418394749, 132556120, 1199103528, 0,
    ],
    [
        0, 0, 54706, 2996985344, 1834629191, 3964963911, 485140318, 1073741824,
    ],
    [
        0, 0, 0, 357981, 1476998812, 3337178590, 1483338760, 4194304000,
    ],
    [0, 0, 0, 0, 2342503, 3052466824, 2595180627, 17825792],
    [0, 0, 0, 0, 0, 15328518, 1933902296, 4063920128],
    [0, 0, 0, 0, 0, 0, 100304420, 3355157504],
    [0, 0, 0, 0, 0, 0, 0, 656356768],
    [0, 0, 0, 0, 0, 0, 0, 1],
];

pub const DEC_TABLE_64: [[u32; BINARY_SZ_64]; INTERMEDIATE_SZ_64] = [
    [
        249448, 3719864065, 173911550, 4021557284, 3115810883, 2498525019, 1035889824, 627529458,
        3840888383, 3728167192, 2901437456, 3863405776, 1540739182, 1570766848, 0, 0,
    ],
    [
        0, 1632305, 1882780341, 4128706713, 1023671068, 2618421812, 2005415586, 1062993857,
        3577221846, 3960476767, 1695615427, 2597060712, 669472826, 104923136, 0, 0,
    ],
    [
        0, 0, 10681231, 1422956801, 2406345166, 4058671871, 2143913881, 4169135587, 2414104418,
        2549553452, 997594232, 713340517, 2290070198, 1103833088, 0, 0,
    ],
    [
        0, 0, 0, 69894212, 1038812943, 1785020643, 1285619000, 2301468615, 3492037905, 314610629,
        2761740102, 3410618104, 1699516363, 910779968, 0, 0,
    ],
    [
        0, 0, 0, 0, 457363084, 927569770, 3976106370, 1389513021, 2107865525, 3716679421,
        1828091393, 2088408376, 439156799, 2579227194, 0, 0,
    ],
    [
        0, 0, 0, 0, 0, 2992822783, 383623235, 3862831115, 112778334, 339767049, 1447250220,
        486575164, 3495303162, 2209946163, 268435456, 0,
    ],
    [
        0, 0, 0, 0, 0, 4, 2404108010, 2962826229, 3998086794, 1893006839, 2266258239, 1429430446,
        307953032, 2361423716, 176160768, 0,
    ],
    [
        0, 0, 0, 0, 0, 0, 29, 3596590989, 3044036677, 1332209423, 1014420882, 868688145,
        4264082837, 3688771808, 2485387264, 0,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 195, 1054003707, 3711696540, 582574436, 3549229270, 1088536814,
        2338440092, 1468637184, 0,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 1277, 2650397687, 3801011509, 2074386530, 3248244966, 687255411,
        2959155456, 0,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 8360, 1184754854, 3047609191, 3418394749, 132556120, 1199103528,
        0,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 54706, 2996985344, 1834629191, 3964963911, 485140318,
        1073741824,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 357981, 1476998812, 3337178590, 1483338760, 4194304000,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2342503, 3052466824, 2595180627, 17825792,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15328518, 1933902296, 4063920128,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100304420, 3355157504,
    ],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 656356768],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
];

#[derive(Debug, PartialEq)]
pub enum DecodeError {
    InvalidChar(u8),
    TooLong,
    TooShort,
    LargestTermTooHigh,
    OutputTooLong,
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

#[cfg(feature = "std")]
impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeError::InvalidChar(c) => {
                write!(f, "Illegal base58 char number: {}", c)
            }
            DecodeError::TooLong => f.write_str("Base58 string too long"),
            DecodeError::TooShort => f.write_str("Base58 string too short"),
            DecodeError::LargestTermTooHigh => f.write_str("Largest term greater than 2^32"),
            DecodeError::OutputTooLong => f.write_str("Decoded output has too many bytes"),
        }
    }
}
