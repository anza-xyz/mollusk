use crate::{
    consts::{ALT_BN128_FIELD_SIZE, ALT_BN128_G1_POINT_SIZE},
    AltBn128Error, LE_FLAG,
};
#[cfg(target_os = "solana")]
use solana_define_syscall::definitions as syscalls;
#[cfg(not(target_os = "solana"))]
use {
    crate::{
        target_arch::{convert_endianness_64, reverse_copy, Endianness, G1},
        PodG1,
    },
    ark_ec::{self, AffineRepr},
    ark_ff::BigInteger256,
    ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress},
};

/// Input size for the multiplication operation.
pub const ALT_BN128_MULTIPLICATION_INPUT_SIZE: usize =
    ALT_BN128_G1_POINT_SIZE + ALT_BN128_FIELD_SIZE; // 96
/// Output size for the multiplication operation.
pub const ALT_BN128_MULTIPLICATION_OUTPUT_SIZE: usize = ALT_BN128_G1_POINT_SIZE; // 64

#[deprecated(
    since = "3.1.0",
    note = "Please use `ALT_BN128_MULTIPLICATION_INPUT_SIZE` instead"
)]
pub const ALT_BN128_MULTIPLICATION_INPUT_LEN: usize = ALT_BN128_MULTIPLICATION_INPUT_SIZE;
#[deprecated(
    since = "3.1.0",
    note = "Please use `ALT_BN128_MULTIPLICATION_OUTPUT_SIZE` instead"
)]
pub const ALT_BN128_MULTIPLICATION_OUTPUT_LEN: usize = ALT_BN128_MULTIPLICATION_OUTPUT_SIZE;

pub const ALT_BN128_G1_MUL_BE: u64 = 2;
#[deprecated(since = "3.1.0", note = "Please use `ALT_BN128_G1_MUL_BE` instead")]
pub const ALT_BN128_MUL: u64 = ALT_BN128_G1_MUL_BE;
pub const ALT_BN128_G1_MUL_LE: u64 = ALT_BN128_G1_MUL_BE | LE_FLAG;

/// The version enum used to version changes to the `alt_bn128_multiplication` syscall.
#[cfg(not(target_os = "solana"))]
pub enum VersionedG1Multiplication {
    V0,
    /// SIMD-0222 - Fix alt-bn128-multiplication Syscall Length Check
    V1,
}

/// The syscall implementation for the `alt_bn128_multiplication` syscall.
///
/// This function is intended to be used by the Agave validator client and exists primarily
/// for validator code. Solana programs or other downstream projects should use
/// `alt_bn128_multiplication` or `alt_bn128_multiplication_le` instead.
///
/// # Warning
///
/// Developers should be extremely careful when modifying this function, as a breaking change
/// can result in a fork in the Solana cluster. Any such change requires an
/// approved Solana SIMD. Subsequently, a new `VersionedMultiplication` variant must be added,
/// and the new logic must be scoped to that variant.
#[cfg(not(target_os = "solana"))]
pub fn alt_bn128_versioned_g1_multiplication(
    version: VersionedG1Multiplication,
    input: &[u8],
    endianness: Endianness,
) -> Result<Vec<u8>, AltBn128Error> {
    let expected_length = match version {
        VersionedG1Multiplication::V0 => 128,
        VersionedG1Multiplication::V1 => ALT_BN128_MULTIPLICATION_INPUT_SIZE,
    };

    match endianness {
        Endianness::BE => {
            if input.len() > expected_length {
                return Err(AltBn128Error::InvalidInputData);
            }
        }
        Endianness::LE => {
            if input.len() != expected_length {
                return Err(AltBn128Error::InvalidInputData);
            }
        }
    }

    let mut input = input.to_vec();
    match endianness {
        Endianness::BE => input.resize(expected_length, 0),
        Endianness::LE => (),
    }

    let p: G1 = match endianness {
        Endianness::BE => PodG1::from_be_bytes(&input[..64])?.try_into()?,
        Endianness::LE => PodG1::from_le_bytes(&input[..64])?.try_into()?,
    };
    let mut fr_bytes = [0u8; 32];
    match endianness {
        Endianness::BE => {
            reverse_copy(&input[64..96], &mut fr_bytes)?;
        }
        Endianness::LE => {
            fr_bytes.copy_from_slice(&input[64..96]);
        }
    }
    let fr = BigInteger256::deserialize_uncompressed_unchecked(fr_bytes.as_slice())
        .map_err(|_| AltBn128Error::InvalidInputData)?;

    let result_point: G1 = p.mul_bigint(fr).into();

    let mut result_point_data = [0u8; ALT_BN128_MULTIPLICATION_OUTPUT_SIZE];

    result_point
        .x
        .serialize_with_mode(&mut result_point_data[..32], Compress::No)
        .map_err(|_| AltBn128Error::InvalidInputData)?;
    result_point
        .y
        .serialize_with_mode(&mut result_point_data[32..], Compress::No)
        .map_err(|_| AltBn128Error::InvalidInputData)?;

    match endianness {
        Endianness::BE => Ok(convert_endianness_64(&result_point_data[..])),
        Endianness::LE => Ok(result_point_data.to_vec()),
    }
}

#[inline(always)]
pub fn alt_bn128_g1_multiplication_be(input: &[u8]) -> Result<Vec<u8>, AltBn128Error> {
    #[cfg(not(target_os = "solana"))]
    {
        alt_bn128_versioned_g1_multiplication(VersionedG1Multiplication::V1, input, Endianness::BE)
    }
    #[cfg(target_os = "solana")]
    {
        if input.len() > ALT_BN128_MULTIPLICATION_INPUT_SIZE {
            return Err(AltBn128Error::InvalidInputData);
        }
        let mut result_buffer = [0u8; ALT_BN128_MULTIPLICATION_OUTPUT_SIZE];
        let result = unsafe {
            syscalls::sol_alt_bn128_group_op(
                ALT_BN128_G1_MUL_BE,
                input as *const _ as *const u8,
                input.len() as u64,
                &mut result_buffer as *mut _ as *mut u8,
            )
        };

        match result {
            0 => Ok(result_buffer.to_vec()),
            _ => Err(AltBn128Error::UnexpectedError),
        }
    }
}

#[deprecated(
    since = "3.1.0",
    note = "Please use `alt_bn128_g1_multiplication_be` instead"
)]
#[inline(always)]
pub fn alt_bn128_multiplication(input: &[u8]) -> Result<Vec<u8>, AltBn128Error> {
    alt_bn128_g1_multiplication_be(input)
}

#[inline(always)]
pub fn alt_bn128_g1_multiplication_le(
    input: &[u8; ALT_BN128_MULTIPLICATION_INPUT_SIZE],
) -> Result<Vec<u8>, AltBn128Error> {
    #[cfg(not(target_os = "solana"))]
    {
        alt_bn128_versioned_g1_multiplication(VersionedG1Multiplication::V1, input, Endianness::LE)
    }
    #[cfg(target_os = "solana")]
    {
        let mut result_buffer = [0u8; ALT_BN128_MULTIPLICATION_OUTPUT_SIZE];
        let result = unsafe {
            syscalls::sol_alt_bn128_group_op(
                ALT_BN128_G1_MUL_LE,
                input as *const _ as *const u8,
                input.len() as u64,
                &mut result_buffer as *mut _ as *mut u8,
            )
        };

        match result {
            0 => Ok(result_buffer.to_vec()),
            _ => Err(AltBn128Error::UnexpectedError),
        }
    }
}

#[deprecated(
    since = "3.1.0",
    note = "Please use `alt_bn128_g1_multiplication_be` instead"
)]
#[cfg(not(target_os = "solana"))]
#[inline(always)]
pub fn alt_bn128_multiplication_128(input: &[u8]) -> Result<Vec<u8>, AltBn128Error> {
    alt_bn128_versioned_g1_multiplication(VersionedG1Multiplication::V0, input, Endianness::BE)
}
