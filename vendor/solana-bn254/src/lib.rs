#![cfg_attr(docsrs, feature(doc_auto_cfg))]
pub(crate) mod addition;
pub mod compression;
pub(crate) mod multiplication;
pub(crate) mod pairing;

/// This module contains the versioned syscall implementations and is intended for use
/// primarily by validator code.
#[cfg(not(target_os = "solana"))]
pub mod versioned {
    pub use crate::{
        addition::{
            alt_bn128_versioned_g1_addition, VersionedG1Addition, ALT_BN128_ADDITION_INPUT_SIZE,
            ALT_BN128_ADDITION_OUTPUT_SIZE, ALT_BN128_G1_ADD_BE, ALT_BN128_G1_ADD_LE,
            ALT_BN128_G1_SUB_BE, ALT_BN128_G1_SUB_LE,
        },
        multiplication::{
            alt_bn128_versioned_g1_multiplication, VersionedG1Multiplication, ALT_BN128_G1_MUL_BE,
            ALT_BN128_G1_MUL_LE, ALT_BN128_MULTIPLICATION_INPUT_SIZE,
            ALT_BN128_MULTIPLICATION_OUTPUT_SIZE,
        },
        pairing::{
            alt_bn128_versioned_pairing, VersionedPairing, ALT_BN128_PAIRING_BE,
            ALT_BN128_PAIRING_ELEMENT_SIZE, ALT_BN128_PAIRING_LE, ALT_BN128_PAIRING_OUTPUT_SIZE,
        },
        target_arch::Endianness,
    };
    #[allow(deprecated)]
    pub use crate::{
        addition::{
            ALT_BN128_ADD, ALT_BN128_ADDITION_INPUT_LEN, ALT_BN128_ADDITION_OUTPUT_LEN,
            ALT_BN128_SUB,
        },
        multiplication::{
            ALT_BN128_MUL, ALT_BN128_MULTIPLICATION_INPUT_LEN, ALT_BN128_MULTIPLICATION_OUTPUT_LEN,
        },
        pairing::{ALT_BN128_PAIRING, ALT_BN128_PAIRING_ELEMENT_LEN, ALT_BN128_PAIRING_OUTPUT_LEN},
    };
}

/// This module should be used by Solana programs or other downstream projects.
pub mod prelude {
    #[allow(deprecated)]
    #[cfg(not(target_os = "solana"))]
    pub use crate::multiplication::alt_bn128_multiplication_128; // to be removed in v4.0
    #[allow(deprecated)]
    pub use crate::{
        addition::{
            alt_bn128_addition, ALT_BN128_ADD, ALT_BN128_ADDITION_INPUT_LEN,
            ALT_BN128_ADDITION_OUTPUT_LEN, ALT_BN128_SUB,
        },
        multiplication::{
            alt_bn128_multiplication, ALT_BN128_MUL, ALT_BN128_MULTIPLICATION_INPUT_LEN,
            ALT_BN128_MULTIPLICATION_OUTPUT_LEN,
        },
        pairing::{
            alt_bn128_pairing, ALT_BN128_PAIRING, ALT_BN128_PAIRING_ELEMENT_LEN,
            ALT_BN128_PAIRING_OUTPUT_LEN,
        },
    };
    pub use crate::{
        addition::{
            alt_bn128_g1_addition_be, alt_bn128_g1_addition_le, ALT_BN128_ADDITION_INPUT_SIZE,
            ALT_BN128_ADDITION_OUTPUT_SIZE, ALT_BN128_G1_ADD_BE, ALT_BN128_G1_ADD_LE,
            ALT_BN128_G1_SUB_BE, ALT_BN128_G1_SUB_LE,
        },
        consts::*,
        multiplication::{
            alt_bn128_g1_multiplication_be, alt_bn128_g1_multiplication_le, ALT_BN128_G1_MUL_BE,
            ALT_BN128_G1_MUL_LE, ALT_BN128_MULTIPLICATION_INPUT_SIZE,
            ALT_BN128_MULTIPLICATION_OUTPUT_SIZE,
        },
        pairing::{
            alt_bn128_pairing_be, alt_bn128_pairing_le, ALT_BN128_PAIRING_BE,
            ALT_BN128_PAIRING_ELEMENT_SIZE, ALT_BN128_PAIRING_LE, ALT_BN128_PAIRING_OUTPUT_SIZE,
        },
        AltBn128Error,
    };
}

#[cfg(not(target_os = "solana"))]
use bytemuck::{Pod, Zeroable};
use thiserror::Error;

mod consts {
    /// Size of the EC point field, in bytes.
    pub const ALT_BN128_FIELD_SIZE: usize = 32;

    /// Size of the EC point. `alt_bn128` point contains
    /// the consistently united x and y fields as 64 bytes.
    pub const ALT_BN128_G1_POINT_SIZE: usize = ALT_BN128_FIELD_SIZE * 2;

    #[deprecated(since = "3.1.0", note = "Please use `ALT_BN128_G1_POINT_SIZE` instead")]
    pub const ALT_BN128_POINT_SIZE: usize = ALT_BN128_G1_POINT_SIZE;

    /// Elements in G2 is represented by 2 field-extension elements `(x, y)`.
    pub const ALT_BN128_G2_POINT_SIZE: usize = ALT_BN128_FIELD_SIZE * 4;
}

// AltBn128Error must be removed once the
// simplify_alt_bn128_syscall_error_codes feature gets activated
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AltBn128Error {
    #[error("The input data is invalid")]
    InvalidInputData,
    #[error("Invalid group data")]
    GroupError,
    #[error("Slice data is going out of input data bounds")]
    SliceOutOfBounds,
    #[error("Unexpected error")]
    UnexpectedError,
    #[error("Failed to convert a byte slice into a vector {0:?}")]
    TryIntoVecError(Vec<u8>),
    #[error("Failed to convert projective to affine g1")]
    ProjectiveToG1Failed,
}

impl From<u64> for AltBn128Error {
    fn from(v: u64) -> AltBn128Error {
        match v {
            1 => AltBn128Error::InvalidInputData,
            2 => AltBn128Error::GroupError,
            3 => AltBn128Error::SliceOutOfBounds,
            4 => AltBn128Error::TryIntoVecError(Vec::new()),
            5 => AltBn128Error::ProjectiveToG1Failed,
            _ => AltBn128Error::UnexpectedError,
        }
    }
}

impl From<AltBn128Error> for u64 {
    fn from(v: AltBn128Error) -> u64 {
        // note: should never return 0, as it risks to be confused with syscall success
        match v {
            AltBn128Error::InvalidInputData => 1,
            AltBn128Error::GroupError => 2,
            AltBn128Error::SliceOutOfBounds => 3,
            AltBn128Error::TryIntoVecError(_) => 4,
            AltBn128Error::ProjectiveToG1Failed => 5,
            AltBn128Error::UnexpectedError => 6,
        }
    }
}

#[cfg(not(target_os = "solana"))]
use consts::{
    ALT_BN128_FIELD_SIZE as FIELD_SIZE, ALT_BN128_G1_POINT_SIZE as G1_POINT_SIZE,
    ALT_BN128_G2_POINT_SIZE as G2_POINT_SIZE,
};

/// A bitmask used to indicate that an operation's input data is little-endian.
pub(crate) const LE_FLAG: u64 = 0x80;

/// The BN254 (BN128) group element in G1 as a POD type.
///
/// A group element in G1 consists of two field elements `(x, y)`. A `PodG1`
/// type expects a group element to be encoded as `[le(x), le(y)]` where
/// `le(..)` is the little-endian encoding of the input field element as used
/// in the `ark-bn254` crate. Note that this differs from the EIP-197 standard,
/// which specifies that the field elements are encoded as big-endian.
///
/// `PodG1` can be constructed from both big-endian (EIP-197) and little-endian
/// (ark-bn254) encodings using `from_be_bytes` and `from_le_bytes` methods,
/// respectively.
#[cfg(not(target_os = "solana"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodG1(pub [u8; G1_POINT_SIZE]);

/// The BN254 (BN128) group element in G2 as a POD type.
///
/// Elements in G2 is represented by 2 field-extension elements `(x, y)`. Each
/// field-extension element itself is a degree 1 polynomial `x = x0 + x1*X`,
/// `y = y0 + y1*X`. The EIP-197 standard encodes a G2 element as
/// `[be(x1), be(x0), be(y1), be(y0)]` where `be(..)` is the big-endian
/// encoding of the input field element. The `ark-bn254` crate encodes a G2
/// element as `[le(x0), le(x1), le(y0), le(y1)]` where `le(..)` is the
/// little-endian encoding of the input field element. Notably, in addition to
/// the differences in the big-endian vs. little-endian encodings of field
/// elements, the order of the polynomial field coefficients `x0`, `x1`, `y0`,
/// and `y1` are different.
///
/// `PodG2` can be constructed from both big-endian (EIP-197) and little-endian
/// (ark-bn254) encodings using `from_be_bytes` and `from_le_bytes` methods,
/// respectively.
#[cfg(not(target_os = "solana"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PodG2(pub [u8; G2_POINT_SIZE]);

#[cfg(not(target_os = "solana"))]
mod target_arch {
    use {
        super::*,
        ark_ec::{self, AffineRepr},
        ark_serialize::{CanonicalDeserialize, Compress, Validate},
    };

    pub(crate) type G1 = ark_bn254::g1::G1Affine;
    pub(crate) type G2 = ark_bn254::g2::G2Affine;

    impl PodG1 {
        /// Takes in an EIP-197 (big-endian) byte encoding of a group element in G1 and constructs a
        /// `PodG1` struct that encodes the same bytes in little-endian.
        pub(crate) fn from_be_bytes(be_bytes: &[u8]) -> Result<Self, AltBn128Error> {
            if be_bytes.len() != G1_POINT_SIZE {
                return Err(AltBn128Error::SliceOutOfBounds);
            }
            let mut pod_bytes = [0u8; G1_POINT_SIZE];
            reverse_copy(&be_bytes[..FIELD_SIZE], &mut pod_bytes[..FIELD_SIZE])?;
            reverse_copy(&be_bytes[FIELD_SIZE..], &mut pod_bytes[FIELD_SIZE..])?;
            Ok(Self(pod_bytes))
        }

        /// Takes in a little-endian byte encoding of a group element in G1 and constructs a
        /// `PodG1` struct that encodes the same bytes internally.
        #[inline(always)]
        pub(crate) fn from_le_bytes(le_bytes: &[u8]) -> Result<Self, AltBn128Error> {
            Ok(Self(
                le_bytes
                    .try_into()
                    .map_err(|_| AltBn128Error::SliceOutOfBounds)?,
            ))
        }
    }

    impl PodG2 {
        /// Takes in an EIP-197 (big-endian) byte encoding of a group element in G2
        /// and constructs a `PodG2` struct that encodes the same bytes in
        /// little-endian.
        pub(crate) fn from_be_bytes(be_bytes: &[u8]) -> Result<Self, AltBn128Error> {
            if be_bytes.len() != G2_POINT_SIZE {
                return Err(AltBn128Error::SliceOutOfBounds);
            }
            // note the cross order
            const SOURCE_X1_INDEX: usize = 0;
            const SOURCE_X0_INDEX: usize = SOURCE_X1_INDEX.saturating_add(FIELD_SIZE);
            const SOURCE_Y1_INDEX: usize = SOURCE_X0_INDEX.saturating_add(FIELD_SIZE);
            const SOURCE_Y0_INDEX: usize = SOURCE_Y1_INDEX.saturating_add(FIELD_SIZE);

            const TARGET_X0_INDEX: usize = 0;
            const TARGET_X1_INDEX: usize = TARGET_X0_INDEX.saturating_add(FIELD_SIZE);
            const TARGET_Y0_INDEX: usize = TARGET_X1_INDEX.saturating_add(FIELD_SIZE);
            const TARGET_Y1_INDEX: usize = TARGET_Y0_INDEX.saturating_add(FIELD_SIZE);

            let mut pod_bytes = [0u8; G2_POINT_SIZE];
            reverse_copy(
                &be_bytes[SOURCE_X1_INDEX..SOURCE_X1_INDEX.saturating_add(FIELD_SIZE)],
                &mut pod_bytes[TARGET_X1_INDEX..TARGET_X1_INDEX.saturating_add(FIELD_SIZE)],
            )?;
            reverse_copy(
                &be_bytes[SOURCE_X0_INDEX..SOURCE_X0_INDEX.saturating_add(FIELD_SIZE)],
                &mut pod_bytes[TARGET_X0_INDEX..TARGET_X0_INDEX.saturating_add(FIELD_SIZE)],
            )?;
            reverse_copy(
                &be_bytes[SOURCE_Y1_INDEX..SOURCE_Y1_INDEX.saturating_add(FIELD_SIZE)],
                &mut pod_bytes[TARGET_Y1_INDEX..TARGET_Y1_INDEX.saturating_add(FIELD_SIZE)],
            )?;
            reverse_copy(
                &be_bytes[SOURCE_Y0_INDEX..SOURCE_Y0_INDEX.saturating_add(FIELD_SIZE)],
                &mut pod_bytes[TARGET_Y0_INDEX..TARGET_Y0_INDEX.saturating_add(FIELD_SIZE)],
            )?;
            Ok(Self(pod_bytes))
        }

        /// Takes in a little-endian byte encoding of a group element in G2 and constructs a
        /// `PodG2` struct that encodes the same bytes internally.
        #[inline(always)]
        pub(crate) fn from_le_bytes(le_bytes: &[u8]) -> Result<Self, AltBn128Error> {
            Ok(Self(
                le_bytes
                    .try_into()
                    .map_err(|_| AltBn128Error::SliceOutOfBounds)?,
            ))
        }
    }

    impl TryFrom<PodG1> for G1 {
        type Error = AltBn128Error;

        fn try_from(bytes: PodG1) -> Result<Self, Self::Error> {
            if bytes.0 == [0u8; 64] {
                return Ok(G1::zero());
            }
            let g1 = Self::deserialize_with_mode(
                &*[&bytes.0[..], &[0u8][..]].concat(),
                Compress::No,
                Validate::Yes,
            );

            match g1 {
                Ok(g1) => {
                    if !g1.is_on_curve() {
                        Err(AltBn128Error::GroupError)
                    } else {
                        Ok(g1)
                    }
                }
                Err(_) => Err(AltBn128Error::InvalidInputData),
            }
        }
    }

    impl TryFrom<PodG2> for G2 {
        type Error = AltBn128Error;

        fn try_from(bytes: PodG2) -> Result<Self, Self::Error> {
            if bytes.0 == [0u8; 128] {
                return Ok(G2::zero());
            }
            let g2 = Self::deserialize_with_mode(
                &*[&bytes.0[..], &[0u8][..]].concat(),
                Compress::No,
                Validate::Yes,
            );

            match g2 {
                Ok(g2) => {
                    if !g2.is_on_curve() {
                        Err(AltBn128Error::GroupError)
                    } else {
                        Ok(g2)
                    }
                }
                Err(_) => Err(AltBn128Error::InvalidInputData),
            }
        }
    }

    pub enum Endianness {
        BE,
        LE,
    }

    pub(crate) fn convert_endianness_64(bytes: &[u8]) -> Vec<u8> {
        bytes
            .chunks(32)
            .flat_map(|b| b.iter().copied().rev().collect::<Vec<u8>>())
            .collect::<Vec<u8>>()
    }

    /// Copies a `source` byte slice into a `destination` byte slice in reverse order.
    pub(crate) fn reverse_copy(source: &[u8], destination: &mut [u8]) -> Result<(), AltBn128Error> {
        if source.len() != destination.len() {
            return Err(AltBn128Error::SliceOutOfBounds);
        }
        for (source_index, destination_index) in source.iter().rev().zip(destination.iter_mut()) {
            *destination_index = *source_index;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::{prelude::*, PodG1},
        ark_bn254::g1::G1Affine,
        ark_ec::AffineRepr,
        ark_serialize::{CanonicalSerialize, Compress},
    };

    #[test]
    fn zero_serialization_test() {
        let zero = G1Affine::zero();
        let mut result_point_data = [0u8; 64];
        zero.x
            .serialize_with_mode(&mut result_point_data[..32], Compress::No)
            .map_err(|_| AltBn128Error::InvalidInputData)
            .unwrap();
        zero.y
            .serialize_with_mode(&mut result_point_data[32..], Compress::No)
            .map_err(|_| AltBn128Error::InvalidInputData)
            .unwrap();
        assert_eq!(result_point_data, [0u8; 64]);

        let p: G1Affine = PodG1(result_point_data[..64].try_into().unwrap())
            .try_into()
            .unwrap();
        assert_eq!(p, zero);
    }
}
