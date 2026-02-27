//! 64-byte signature type.
#![no_std]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(feature = "frozen-abi", feature(min_specialization))]
#[cfg(any(test, feature = "verify"))]
use core::convert::TryInto;
use core::{
    fmt,
    str::{from_utf8_unchecked, FromStr},
};
#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;
use core::error::Error;
#[cfg(feature = "std")]
use std::vec::Vec;
#[cfg(feature = "serde")]
use {
    serde_big_array::BigArray,
    serde_derive::{Deserialize, Serialize},
};

pub mod error;

/// Number of bytes in a signature
pub const SIGNATURE_BYTES: usize = 64;
/// Maximum string length of a base58 encoded signature
const MAX_BASE58_SIGNATURE_LEN: usize = 88;

#[repr(transparent)]
#[cfg_attr(feature = "frozen-abi", derive(solana_frozen_abi_macro::AbiExample))]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(
    feature = "bytemuck",
    derive(bytemuck_derive::Pod, bytemuck_derive::Zeroable)
)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Signature(
    #[cfg_attr(feature = "serde", serde(with = "BigArray"))] [u8; SIGNATURE_BYTES],
);

impl Default for Signature {
    fn default() -> Self {
        Self([0u8; 64])
    }
}

impl solana_sanitize::Sanitize for Signature {}

impl Signature {
    /// Return a reference to the `Signature`'s byte array.
    #[inline(always)]
    pub const fn as_array(&self) -> &[u8; SIGNATURE_BYTES] {
        &self.0
    }
}

#[cfg(feature = "rand")]
impl Signature {
    pub fn new_unique() -> Self {
        Self::from(core::array::from_fn(|_| rand::random()))
    }
}

#[cfg(any(test, feature = "verify"))]
impl Signature {
    pub(self) fn verify_verbose(
        &self,
        pubkey_bytes: &[u8],
        message_bytes: &[u8],
    ) -> Result<(), ed25519_dalek::SignatureError> {
        let publickey = ed25519_dalek::VerifyingKey::try_from(pubkey_bytes)?;
        let signature = self.0.as_slice().try_into()?;
        publickey.verify_strict(message_bytes, &signature)
    }

    pub fn verify(&self, pubkey_bytes: &[u8], message_bytes: &[u8]) -> bool {
        self.verify_verbose(pubkey_bytes, message_bytes).is_ok()
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

fn write_as_base58(f: &mut fmt::Formatter, s: &Signature) -> fmt::Result {
    let mut out = [0u8; MAX_BASE58_SIGNATURE_LEN];
    let len = five8::encode_64(&s.0, &mut out) as usize;
    // any sequence of base58 chars is valid utf8
    let as_str = unsafe { from_utf8_unchecked(&out[..len]) };
    f.write_str(as_str)
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_as_base58(f, self)
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_as_base58(f, self)
    }
}

impl From<Signature> for [u8; 64] {
    fn from(signature: Signature) -> Self {
        signature.0
    }
}

impl From<[u8; SIGNATURE_BYTES]> for Signature {
    #[inline]
    fn from(signature: [u8; SIGNATURE_BYTES]) -> Self {
        Self(signature)
    }
}

impl<'a> TryFrom<&'a [u8]> for Signature {
    type Error = <[u8; SIGNATURE_BYTES] as TryFrom<&'a [u8]>>::Error;

    #[inline]
    fn try_from(signature: &'a [u8]) -> Result<Self, Self::Error> {
        <[u8; SIGNATURE_BYTES]>::try_from(signature).map(Self::from)
    }
}

#[cfg(feature = "std")]
impl TryFrom<Vec<u8>> for Signature {
    type Error = <[u8; SIGNATURE_BYTES] as TryFrom<Vec<u8>>>::Error;

    #[inline]
    fn try_from(signature: Vec<u8>) -> Result<Self, Self::Error> {
        <[u8; SIGNATURE_BYTES]>::try_from(signature).map(Self::from)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseSignatureError {
    WrongSize,
    Invalid,
}

impl Error for ParseSignatureError {}

impl fmt::Display for ParseSignatureError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseSignatureError::WrongSize => {
                f.write_str("string decoded to wrong size for signature")
            }
            ParseSignatureError::Invalid => f.write_str("failed to decode string to signature"),
        }
    }
}

impl FromStr for Signature {
    type Err = ParseSignatureError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use five8::DecodeError;
        if s.len() > MAX_BASE58_SIGNATURE_LEN {
            return Err(ParseSignatureError::WrongSize);
        }
        let mut bytes = [0; SIGNATURE_BYTES];
        five8::decode_64(s, &mut bytes).map_err(|e| match e {
            DecodeError::InvalidChar(_) => ParseSignatureError::Invalid,
            DecodeError::TooLong
            | DecodeError::TooShort
            | DecodeError::LargestTermTooHigh
            | DecodeError::OutputTooLong => ParseSignatureError::WrongSize,
        })?;
        Ok(Self::from(bytes))
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        serde_derive::{Deserialize, Serialize},
        solana_pubkey::Pubkey,
    };

    #[test]
    fn test_off_curve_pubkey_verify_fails() {
        // Golden point off the ed25519 curve
        let off_curve_bytes = bs58::decode("9z5nJyQar1FUxVJxpBXzon6kHehbomeYiDaLi9WAMhCq")
            .into_vec()
            .unwrap();

        // Confirm golden's off-curvedness
        let mut off_curve_bits = [0u8; 32];
        off_curve_bits.copy_from_slice(&off_curve_bytes);
        let off_curve_point = curve25519_dalek::edwards::CompressedEdwardsY(off_curve_bits);
        assert_eq!(off_curve_point.decompress(), None);

        let pubkey = Pubkey::try_from(off_curve_bytes).unwrap();
        let signature = Signature::default();
        // Unfortunately, ed25519-dalek doesn't surface the internal error types that we'd ideally
        // `source()` out of the `SignatureError` returned by `verify_strict()`.  So the best we
        // can do is `is_err()` here.
        assert!(signature.verify_verbose(pubkey.as_ref(), &[0u8]).is_err());
    }

    #[test]
    fn test_short_vec() {
        #[derive(Debug, Deserialize, Serialize, PartialEq)]
        struct SigShortVec {
            #[serde(with = "solana_short_vec")]
            pub signatures: Vec<Signature>,
        }
        let sig = Signature::from([
            120, 138, 162, 185, 59, 209, 241, 157, 71, 157, 74, 131, 4, 87, 54, 28, 38, 180, 222,
            82, 64, 62, 61, 62, 22, 46, 17, 203, 187, 136, 62, 43, 11, 38, 235, 17, 239, 82, 240,
            139, 130, 217, 227, 214, 9, 242, 141, 223, 94, 29, 184, 110, 62, 32, 87, 137, 63, 139,
            100, 221, 20, 137, 4, 5,
        ]);
        let to_serialize = SigShortVec {
            signatures: std::vec![sig],
        };
        let json_serialized = serde_json::to_string(&to_serialize).unwrap();
        assert_eq!(json_serialized, "{\"signatures\":[[1],[120,138,162,185,59,209,241,157,71,157,74,131,4,87,54,28,38,180,222,82,64,62,61,62,22,46,17,203,187,136,62,43,11,38,235,17,239,82,240,139,130,217,227,214,9,242,141,223,94,29,184,110,62,32,87,137,63,139,100,221,20,137,4,5]]}");
        let json_deserialized: SigShortVec = serde_json::from_str(&json_serialized).unwrap();
        assert_eq!(json_deserialized, to_serialize);
        let bincode_serialized = bincode::serialize(&to_serialize).unwrap();
        assert_eq!(
            bincode_serialized,
            [
                1, 120, 138, 162, 185, 59, 209, 241, 157, 71, 157, 74, 131, 4, 87, 54, 28, 38, 180,
                222, 82, 64, 62, 61, 62, 22, 46, 17, 203, 187, 136, 62, 43, 11, 38, 235, 17, 239,
                82, 240, 139, 130, 217, 227, 214, 9, 242, 141, 223, 94, 29, 184, 110, 62, 32, 87,
                137, 63, 139, 100, 221, 20, 137, 4, 5
            ]
        );
        let bincode_deserialized: SigShortVec = bincode::deserialize(&bincode_serialized).unwrap();
        assert_eq!(bincode_deserialized, to_serialize);
    }

    #[test]
    fn test_signature_fromstr() {
        let signature = Signature::from([
            103, 7, 88, 96, 203, 140, 191, 47, 231, 37, 30, 220, 61, 35, 93, 112, 225, 2, 5, 11,
            158, 105, 246, 147, 133, 64, 109, 252, 119, 73, 108, 248, 167, 240, 160, 18, 222, 3, 1,
            48, 51, 67, 94, 19, 91, 108, 227, 126, 100, 25, 212, 135, 90, 60, 61, 78, 186, 104, 22,
            58, 242, 74, 148, 6,
        ]);

        let mut signature_base58_str = bs58::encode(signature).into_string();

        assert_eq!(signature_base58_str.parse::<Signature>(), Ok(signature));

        signature_base58_str.push_str(&bs58::encode(<[u8; 64]>::from(signature)).into_string());
        assert_eq!(
            signature_base58_str.parse::<Signature>(),
            Err(ParseSignatureError::WrongSize)
        );

        signature_base58_str.truncate(signature_base58_str.len() / 2);
        assert_eq!(signature_base58_str.parse::<Signature>(), Ok(signature));

        signature_base58_str.truncate(signature_base58_str.len() / 2);
        assert_eq!(
            signature_base58_str.parse::<Signature>(),
            Err(ParseSignatureError::WrongSize)
        );

        let mut signature_base58_str = bs58::encode(<[u8; 64]>::from(signature)).into_string();
        assert_eq!(signature_base58_str.parse::<Signature>(), Ok(signature));

        // throw some non-base58 stuff in there
        signature_base58_str.replace_range(..1, "I");
        assert_eq!(
            signature_base58_str.parse::<Signature>(),
            Err(ParseSignatureError::Invalid)
        );

        // too long input string
        // longest valid encoding
        let mut too_long = bs58::encode(&[255u8; SIGNATURE_BYTES]).into_string();
        // and one to grow on
        too_long.push('1');
        assert_eq!(
            too_long.parse::<Signature>(),
            Err(ParseSignatureError::WrongSize)
        );
    }

    #[test]
    fn test_as_array() {
        let bytes = [1u8; 64];
        let signature = Signature::from(bytes);
        assert_eq!(signature.as_array(), &bytes);
        assert_eq!(
            signature.as_array(),
            &<Signature as Into<[u8; 64]>>::into(signature)
        );
        // Sanity check: ensure the pointer is the same.
        assert_eq!(signature.as_array().as_ptr(), signature.0.as_ptr());
    }
}
