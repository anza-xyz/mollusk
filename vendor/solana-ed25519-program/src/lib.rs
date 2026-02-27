//! Instructions for the [ed25519 native program][np].
//!
//! [np]: https://docs.solanalabs.com/runtime/programs#ed25519-program

use {
    bytemuck::bytes_of,
    bytemuck_derive::{Pod, Zeroable},
    solana_instruction::Instruction,
};

pub const PUBKEY_SERIALIZED_SIZE: usize = 32;
pub const SIGNATURE_SERIALIZED_SIZE: usize = 64;
pub const SIGNATURE_OFFSETS_SERIALIZED_SIZE: usize = 14;
// bytemuck requires structures to be aligned
pub const SIGNATURE_OFFSETS_START: usize = 2;
pub const DATA_START: usize = SIGNATURE_OFFSETS_SERIALIZED_SIZE + SIGNATURE_OFFSETS_START;

#[derive(Default, Debug, Copy, Clone, Zeroable, Pod, Eq, PartialEq)]
#[repr(C)]
pub struct Ed25519SignatureOffsets {
    pub signature_offset: u16, // offset to ed25519 signature of 64 bytes
    pub signature_instruction_index: u16, // instruction index to find signature
    pub public_key_offset: u16, // offset to public key of 32 bytes
    pub public_key_instruction_index: u16, // instruction index to find public key
    pub message_data_offset: u16, // offset to start of message data
    pub message_data_size: u16, // size of message data
    pub message_instruction_index: u16, // index of instruction data to get message data
}

/// Encode just the signature offsets in a single ed25519 instruction.
///
/// This is a convenience function for rare cases where we wish to verify multiple messages in
/// the same instruction. The verification data can be stored in a separate instruction specified
/// by the `*_instruction_index` fields of `offsets`, or in this instruction by extending the data
/// buffer.
///
/// Note: If the signer for these messages are the same, it is cheaper to concatenate the messages
/// and have the signer sign the single buffer and use [`new_ed25519_instruction_with_signature`].
pub fn offsets_to_ed25519_instruction(offsets: &[Ed25519SignatureOffsets]) -> Instruction {
    let mut instruction_data = Vec::with_capacity(
        SIGNATURE_OFFSETS_START
            .saturating_add(SIGNATURE_OFFSETS_SERIALIZED_SIZE.saturating_mul(offsets.len())),
    );

    let num_signatures = offsets.len() as u16;
    instruction_data.extend_from_slice(&num_signatures.to_le_bytes());

    for offsets in offsets {
        instruction_data.extend_from_slice(bytes_of(offsets));
    }

    Instruction {
        program_id: solana_sdk_ids::ed25519_program::id(),
        accounts: vec![],
        data: instruction_data,
    }
}

pub fn new_ed25519_instruction_with_signature(
    message: &[u8],
    signature: &[u8; SIGNATURE_SERIALIZED_SIZE],
    pubkey: &[u8; PUBKEY_SERIALIZED_SIZE],
) -> Instruction {
    let mut instruction_data = Vec::with_capacity(
        DATA_START
            .saturating_add(SIGNATURE_SERIALIZED_SIZE)
            .saturating_add(PUBKEY_SERIALIZED_SIZE)
            .saturating_add(message.len()),
    );

    let num_signatures: u8 = 1;
    let public_key_offset = DATA_START;
    let signature_offset = public_key_offset.saturating_add(PUBKEY_SERIALIZED_SIZE);
    let message_data_offset = signature_offset.saturating_add(SIGNATURE_SERIALIZED_SIZE);

    // add padding byte so that offset structure is aligned
    instruction_data.extend_from_slice(bytes_of(&[num_signatures, 0]));

    let offsets = Ed25519SignatureOffsets {
        signature_offset: signature_offset as u16,
        signature_instruction_index: u16::MAX,
        public_key_offset: public_key_offset as u16,
        public_key_instruction_index: u16::MAX,
        message_data_offset: message_data_offset as u16,
        message_data_size: message.len() as u16,
        message_instruction_index: u16::MAX,
    };

    instruction_data.extend_from_slice(bytes_of(&offsets));

    debug_assert_eq!(instruction_data.len(), public_key_offset);

    instruction_data.extend_from_slice(pubkey);

    debug_assert_eq!(instruction_data.len(), signature_offset);

    instruction_data.extend_from_slice(signature);

    debug_assert_eq!(instruction_data.len(), message_data_offset);

    instruction_data.extend_from_slice(message);

    Instruction {
        program_id: solana_sdk_ids::ed25519_program::id(),
        accounts: vec![],
        data: instruction_data,
    }
}
