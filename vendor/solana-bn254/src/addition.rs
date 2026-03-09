use crate::{consts::ALT_BN128_G1_POINT_SIZE, AltBn128Error, LE_FLAG};
#[cfg(target_os = "solana")]
use solana_define_syscall::definitions as syscalls;
#[cfg(not(target_os = "solana"))]
use {
    crate::{
        target_arch::{convert_endianness_64, Endianness, G1},
        PodG1,
    },
    ark_serialize::{CanonicalSerialize, Compress},
};

/// Input size for the add operation.
pub const ALT_BN128_ADDITION_INPUT_SIZE: usize = ALT_BN128_G1_POINT_SIZE * 2; // 128
/// Output size for the add operation.
pub const ALT_BN128_ADDITION_OUTPUT_SIZE: usize = ALT_BN128_G1_POINT_SIZE; // 64

#[deprecated(
    since = "3.1.0",
    note = "Please use `ALT_BN128_ADDITION_INPUT_SIZE` instead"
)]
pub const ALT_BN128_ADDITION_INPUT_LEN: usize = ALT_BN128_ADDITION_INPUT_SIZE;
#[deprecated(
    since = "3.1.0",
    note = "Please use `ALT_BN128_ADDITION_OUTPUT_SIZE` instead"
)]
pub const ALT_BN128_ADDITION_OUTPUT_LEN: usize = ALT_BN128_ADDITION_OUTPUT_SIZE;

pub const ALT_BN128_G1_ADD_BE: u64 = 0;
pub const ALT_BN128_G1_SUB_BE: u64 = 1;
#[deprecated(since = "3.1.0", note = "Please use `ALT_BN128_G1_ADD_BE` instead")]
pub const ALT_BN128_ADD: u64 = ALT_BN128_G1_ADD_BE;
#[deprecated(since = "3.1.0", note = "Please use `ALT_BN128_G1_SUB_BE` instead")]
pub const ALT_BN128_SUB: u64 = ALT_BN128_G1_SUB_BE;
pub const ALT_BN128_G1_ADD_LE: u64 = ALT_BN128_G1_ADD_BE | LE_FLAG;
pub const ALT_BN128_G1_SUB_LE: u64 = ALT_BN128_G1_SUB_BE | LE_FLAG;

/// The version enum used to version changes to the `alt_bn128_addition` syscall.
#[cfg(not(target_os = "solana"))]
pub enum VersionedG1Addition {
    V0,
}

/// The syscall implementation for the `alt_bn128_addition` syscall.
///
/// This function is intended to be used by the Agave validator client and exists primarily
/// for validator code. Solana programs or other downstream projects should use
/// `alt_bn128_addition` or `alt_bn128_addition_le` instead.
///
/// # Warning
///
/// Developers should be extremely careful when modifying this function, as a breaking change
/// can result in a fork in the Solana cluster. Any such change requires an
/// approved Solana SIMD. Subsequently, a new `VersionedAddition` variant must be added,
/// and the new logic must be scoped to that variant.
#[cfg(not(target_os = "solana"))]
pub fn alt_bn128_versioned_g1_addition(
    _version: VersionedG1Addition,
    input: &[u8],
    endianness: Endianness,
) -> Result<Vec<u8>, AltBn128Error> {
    match endianness {
        Endianness::BE => {
            if input.len() > ALT_BN128_ADDITION_INPUT_SIZE {
                return Err(AltBn128Error::InvalidInputData);
            }
        }
        Endianness::LE => {
            if input.len() != ALT_BN128_ADDITION_INPUT_SIZE {
                return Err(AltBn128Error::InvalidInputData);
            }
        }
    }

    let mut input = input.to_vec();
    match endianness {
        Endianness::BE => input.resize(ALT_BN128_ADDITION_INPUT_SIZE, 0),
        Endianness::LE => (),
    }

    let p: G1 = match endianness {
        Endianness::BE => PodG1::from_be_bytes(&input[..64])?.try_into()?,
        Endianness::LE => PodG1::from_le_bytes(&input[..64])?.try_into()?,
    };

    let q: G1 = match endianness {
        Endianness::BE => PodG1::from_be_bytes(&input[64..])?.try_into()?,
        Endianness::LE => PodG1::from_le_bytes(&input[64..])?.try_into()?,
    };

    #[allow(clippy::arithmetic_side_effects)]
    let result_point = p + q;

    let mut result_point_data = [0u8; ALT_BN128_ADDITION_OUTPUT_SIZE];
    let result_point_affine: G1 = result_point.into();
    result_point_affine
        .x
        .serialize_with_mode(&mut result_point_data[..32], Compress::No)
        .map_err(|_| AltBn128Error::InvalidInputData)?;
    result_point_affine
        .y
        .serialize_with_mode(&mut result_point_data[32..], Compress::No)
        .map_err(|_| AltBn128Error::InvalidInputData)?;

    match endianness {
        Endianness::BE => Ok(convert_endianness_64(&result_point_data[..])),
        Endianness::LE => Ok(result_point_data.to_vec()),
    }
}

#[inline(always)]
pub fn alt_bn128_g1_addition_be(input: &[u8]) -> Result<Vec<u8>, AltBn128Error> {
    #[cfg(not(target_os = "solana"))]
    {
        alt_bn128_versioned_g1_addition(VersionedG1Addition::V0, input, Endianness::BE)
    }
    #[cfg(target_os = "solana")]
    {
        if input.len() > ALT_BN128_ADDITION_INPUT_SIZE {
            return Err(AltBn128Error::InvalidInputData);
        }
        let mut result_buffer = [0; ALT_BN128_ADDITION_OUTPUT_SIZE];
        let result = unsafe {
            syscalls::sol_alt_bn128_group_op(
                ALT_BN128_G1_ADD_BE,
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
    note = "Please use `alt_bn128_g1_addition_be` instead"
)]
#[inline(always)]
pub fn alt_bn128_addition(input: &[u8]) -> Result<Vec<u8>, AltBn128Error> {
    alt_bn128_g1_addition_be(input)
}

#[inline(always)]
pub fn alt_bn128_g1_addition_le(
    input: &[u8; ALT_BN128_ADDITION_INPUT_SIZE],
) -> Result<Vec<u8>, AltBn128Error> {
    #[cfg(not(target_os = "solana"))]
    {
        alt_bn128_versioned_g1_addition(VersionedG1Addition::V0, input, Endianness::LE)
    }
    #[cfg(target_os = "solana")]
    {
        let mut result_buffer = [0; ALT_BN128_ADDITION_OUTPUT_SIZE];
        let result = unsafe {
            syscalls::sol_alt_bn128_group_op(
                ALT_BN128_G1_ADD_LE,
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
