#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(feature = "frozen-abi", feature(min_specialization))]
#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
#[cfg(feature = "std")]
extern crate std;
#[cfg(feature = "bytemuck")]
use bytemuck_derive::{Pod, Zeroable};
#[cfg(feature = "decode")]
use core::{
    fmt,
    str::{from_utf8_unchecked, FromStr},
};
#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};
#[cfg(feature = "sanitize")]
use solana_sanitize::Sanitize;
#[cfg(feature = "borsh")]
extern crate alloc;
#[cfg(feature = "borsh")]
use alloc::string::ToString;

/// Size of a hash in bytes.
pub const HASH_BYTES: usize = 32;
/// Maximum string length of a base58 encoded hash.
pub const MAX_BASE58_LEN: usize = 44;

/// A hash; the 32-byte output of a hashing algorithm.
///
/// This struct is used most often in `solana-sdk` and related crates to contain
/// a [SHA-256] hash, but may instead contain a [blake3] hash.
///
/// [SHA-256]: https://en.wikipedia.org/wiki/SHA-2
/// [blake3]: https://github.com/BLAKE3-team/BLAKE3
#[cfg_attr(feature = "frozen-abi", derive(solana_frozen_abi_macro::AbiExample))]
#[cfg_attr(
    feature = "borsh",
    derive(BorshSerialize, BorshDeserialize),
    borsh(crate = "borsh")
)]
#[cfg_attr(feature = "borsh", derive(BorshSchema))]
#[cfg_attr(feature = "bytemuck", derive(Pod, Zeroable))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize,))]
#[cfg_attr(feature = "copy", derive(Copy))]
#[cfg_attr(not(feature = "decode"), derive(Debug))]
#[derive(Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct Hash(pub(crate) [u8; HASH_BYTES]);

#[cfg(feature = "sanitize")]
impl Sanitize for Hash {}

impl From<[u8; HASH_BYTES]> for Hash {
    fn from(from: [u8; 32]) -> Self {
        Self(from)
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

#[cfg(feature = "decode")]
fn write_as_base58(f: &mut fmt::Formatter, h: &Hash) -> fmt::Result {
    let mut out = [0u8; MAX_BASE58_LEN];
    let len = five8::encode_32(&h.0, &mut out) as usize;
    // any sequence of base58 chars is valid utf8
    let as_str = unsafe { from_utf8_unchecked(&out[..len]) };
    f.write_str(as_str)
}

#[cfg(feature = "decode")]
impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_as_base58(f, self)
    }
}

#[cfg(feature = "decode")]
impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_as_base58(f, self)
    }
}

#[cfg(feature = "decode")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseHashError {
    WrongSize,
    Invalid,
}

#[cfg(feature = "decode")]
impl core::error::Error for ParseHashError {}

#[cfg(feature = "decode")]
impl fmt::Display for ParseHashError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseHashError::WrongSize => f.write_str("string decoded to wrong size for hash"),
            ParseHashError::Invalid => f.write_str("failed to decoded string to hash"),
        }
    }
}

#[cfg(feature = "decode")]
impl FromStr for Hash {
    type Err = ParseHashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use five8::DecodeError;
        if s.len() > MAX_BASE58_LEN {
            return Err(ParseHashError::WrongSize);
        }
        let mut bytes = [0; HASH_BYTES];
        five8::decode_32(s, &mut bytes).map_err(|e| match e {
            DecodeError::InvalidChar(_) => ParseHashError::Invalid,
            DecodeError::TooLong
            | DecodeError::TooShort
            | DecodeError::LargestTermTooHigh
            | DecodeError::OutputTooLong => ParseHashError::WrongSize,
        })?;
        Ok(Self::from(bytes))
    }
}

impl Hash {
    pub const fn new_from_array(hash_array: [u8; HASH_BYTES]) -> Self {
        Self(hash_array)
    }

    /// unique Hash for tests and benchmarks.
    #[cfg(feature = "atomic")]
    pub fn new_unique() -> Self {
        use solana_atomic_u64::AtomicU64;
        static I: AtomicU64 = AtomicU64::new(1);

        let mut b = [0u8; HASH_BYTES];
        let i = I.fetch_add(1);
        b[0..8].copy_from_slice(&i.to_le_bytes());
        Self::new_from_array(b)
    }

    pub const fn to_bytes(&self) -> [u8; HASH_BYTES] {
        self.0
    }

    pub const fn as_bytes(&self) -> &[u8; HASH_BYTES] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_unique() {
        assert!(Hash::new_unique() != Hash::new_unique());
    }

    #[test]
    fn test_hash_fromstr() {
        let hash = Hash::new_from_array([1; 32]);

        let mut hash_base58_str = bs58::encode(hash).into_string();

        assert_eq!(hash_base58_str.parse::<Hash>(), Ok(hash));

        hash_base58_str.push_str(&bs58::encode(hash.as_ref()).into_string());
        assert_eq!(
            hash_base58_str.parse::<Hash>(),
            Err(ParseHashError::WrongSize)
        );

        hash_base58_str.truncate(hash_base58_str.len() / 2);
        assert_eq!(hash_base58_str.parse::<Hash>(), Ok(hash));

        hash_base58_str.truncate(hash_base58_str.len() / 2);
        assert_eq!(
            hash_base58_str.parse::<Hash>(),
            Err(ParseHashError::WrongSize)
        );

        let input_too_big = bs58::encode(&[0xffu8; HASH_BYTES + 1]).into_string();
        assert!(input_too_big.len() > MAX_BASE58_LEN);
        assert_eq!(
            input_too_big.parse::<Hash>(),
            Err(ParseHashError::WrongSize)
        );

        let mut hash_base58_str = bs58::encode(hash.as_ref()).into_string();
        assert_eq!(hash_base58_str.parse::<Hash>(), Ok(hash));

        // throw some non-base58 stuff in there
        hash_base58_str.replace_range(..1, "I");
        assert_eq!(
            hash_base58_str.parse::<Hash>(),
            Err(ParseHashError::Invalid)
        );
    }
}
