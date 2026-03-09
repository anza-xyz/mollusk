//! Instructions for the
//! [secp256r1 native program](https://docs.solana.com/developing/runtime-facilities/programs#secp256r1-program)
//!
//! Note on Signature Malleability:
//! This precompile requires low-S values in signatures (s <= half_curve_order) to prevent signature malleability.
//! Signature malleability means that for a valid signature (r,s), (r, order-s) is also valid for the
//! same message and public key.
//!
//! This property can be problematic for developers who assume each signature is unique. Without enforcing
//! low-S values, the same message and key can produce two different valid signatures, potentially breaking
//! replay protection schemes that rely on signature uniqueness.
use bytemuck::{Pod, Zeroable};
pub use solana_sdk_ids::secp256r1_program::{check_id, id, ID};

#[derive(Default, Debug, Copy, Clone, Zeroable, Pod, Eq, PartialEq)]
#[repr(C)]
pub struct Secp256r1SignatureOffsets {
    /// Offset to compact secp256r1 signature of 64 bytes
    pub signature_offset: u16,

    /// Instruction index where the signature can be found
    pub signature_instruction_index: u16,

    /// Offset to compressed public key of 33 bytes
    pub public_key_offset: u16,

    /// Instruction index where the public key can be found
    pub public_key_instruction_index: u16,

    /// Offset to the start of message data
    pub message_data_offset: u16,

    /// Size of message data in bytes
    pub message_data_size: u16,

    /// Instruction index where the message data can be found
    pub message_instruction_index: u16,
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "solana")))]
mod target_arch {
    use {
        crate::Secp256r1SignatureOffsets,
        bytemuck::bytes_of,
        openssl::{bn::BigNum, ec::EcKey, ecdsa::EcdsaSig, nid::Nid, pkey::PKey, sign::Signer},
        solana_instruction::Instruction,
    };

    pub const COMPRESSED_PUBKEY_SERIALIZED_SIZE: usize = 33;
    pub const SIGNATURE_SERIALIZED_SIZE: usize = 64;
    pub const SIGNATURE_OFFSETS_SERIALIZED_SIZE: usize = 14;
    pub const SIGNATURE_OFFSETS_START: usize = 2;
    pub const DATA_START: usize = SIGNATURE_OFFSETS_SERIALIZED_SIZE + SIGNATURE_OFFSETS_START;

    // Order as defined in SEC2: 2.7.2 Recommended Parameters secp256r1
    pub const SECP256R1_ORDER: [u8; FIELD_SIZE] = [
        0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xBC, 0xE6, 0xFA, 0xAD, 0xA7, 0x17, 0x9E, 0x84, 0xF3, 0xB9, 0xCA, 0xC2, 0xFC, 0x63,
        0x25, 0x51,
    ];

    // Computed SECP256R1_ORDER - 1
    pub const SECP256R1_ORDER_MINUS_ONE: [u8; FIELD_SIZE] = [
        0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xBC, 0xE6, 0xFA, 0xAD, 0xA7, 0x17, 0x9E, 0x84, 0xF3, 0xB9, 0xCA, 0xC2, 0xFC, 0x63,
        0x25, 0x50,
    ];

    // Computed half order
    pub const SECP256R1_HALF_ORDER: [u8; FIELD_SIZE] = [
        0x7F, 0xFF, 0xFF, 0xFF, 0x80, 0x00, 0x00, 0x00, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xDE, 0x73, 0x7D, 0x56, 0xD3, 0x8B, 0xCF, 0x42, 0x79, 0xDC, 0xE5, 0x61, 0x7E, 0x31,
        0x92, 0xA8,
    ];
    // Field size in bytes
    pub const FIELD_SIZE: usize = 32;

    pub fn sign_message(
        message: &[u8],
        priv_key_bytes_der: &[u8],
    ) -> Result<[u8; SIGNATURE_SERIALIZED_SIZE], Box<dyn core::error::Error>> {
        let signing_key = EcKey::private_key_from_der(priv_key_bytes_der)?;
        if signing_key.group().curve_name() != Some(Nid::X9_62_PRIME256V1) {
            return Err(("Signing key must be on the secp256r1 curve".to_string()).into());
        }

        let signing_key_pkey = PKey::from_ec_key(signing_key)?;

        let mut signer = Signer::new(openssl::hash::MessageDigest::sha256(), &signing_key_pkey)?;
        signer.update(message)?;
        let signature = signer.sign_to_vec()?;

        let ecdsa_sig = EcdsaSig::from_der(&signature)?;
        let r = ecdsa_sig.r().to_vec();
        let s = ecdsa_sig.s().to_vec();
        let mut signature = [0u8; SIGNATURE_SERIALIZED_SIZE];

        // Incase of an r or s value of 31 bytes we need to pad it to 32 bytes
        let mut padded_r = [0u8; FIELD_SIZE];
        let mut padded_s = [0u8; FIELD_SIZE];
        padded_r[FIELD_SIZE.saturating_sub(r.len())..].copy_from_slice(&r);
        padded_s[FIELD_SIZE.saturating_sub(s.len())..].copy_from_slice(&s);

        signature[..FIELD_SIZE].copy_from_slice(&padded_r);
        signature[FIELD_SIZE..].copy_from_slice(&padded_s);

        // Check if s > half_order, if so, compute s = order - s
        let s_bignum = BigNum::from_slice(&s)?;
        let half_order = BigNum::from_slice(&SECP256R1_HALF_ORDER)?;
        let order = BigNum::from_slice(&SECP256R1_ORDER)?;
        if s_bignum > half_order {
            let mut new_s = BigNum::new()?;
            new_s.checked_sub(&order, &s_bignum)?;
            let new_s_bytes = new_s.to_vec();

            // Incase the new s value is 31 bytes we need to pad it to 32 bytes
            let mut new_padded_s = [0u8; FIELD_SIZE];
            new_padded_s[FIELD_SIZE.saturating_sub(new_s_bytes.len())..]
                .copy_from_slice(&new_s_bytes);

            signature[FIELD_SIZE..].copy_from_slice(&new_padded_s);
        }
        Ok(signature)
    }

    pub fn new_secp256r1_instruction_with_signature(
        message: &[u8],
        signature: &[u8; SIGNATURE_SERIALIZED_SIZE],
        pubkey: &[u8; COMPRESSED_PUBKEY_SERIALIZED_SIZE],
    ) -> Instruction {
        let mut instruction_data = Vec::with_capacity(
            DATA_START
                .saturating_add(SIGNATURE_SERIALIZED_SIZE)
                .saturating_add(COMPRESSED_PUBKEY_SERIALIZED_SIZE)
                .saturating_add(message.len()),
        );

        let num_signatures: u8 = 1;
        let public_key_offset = DATA_START;
        let signature_offset = public_key_offset.saturating_add(COMPRESSED_PUBKEY_SERIALIZED_SIZE);
        let message_data_offset = signature_offset.saturating_add(SIGNATURE_SERIALIZED_SIZE);

        instruction_data.extend_from_slice(bytes_of(&[num_signatures, 0]));

        let offsets = Secp256r1SignatureOffsets {
            signature_offset: signature_offset as u16,
            signature_instruction_index: u16::MAX,
            public_key_offset: public_key_offset as u16,
            public_key_instruction_index: u16::MAX,
            message_data_offset: message_data_offset as u16,
            message_data_size: message.len() as u16,
            message_instruction_index: u16::MAX,
        };

        instruction_data.extend_from_slice(bytes_of(&offsets));
        instruction_data.extend_from_slice(pubkey);
        instruction_data.extend_from_slice(signature);
        instruction_data.extend_from_slice(message);

        Instruction {
            program_id: crate::id(),
            accounts: vec![],
            data: instruction_data,
        }
    }
}

pub use self::target_arch::*;
