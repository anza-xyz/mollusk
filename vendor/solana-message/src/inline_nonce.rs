//! Inlined nonce instruction information to avoid a dependency on bincode and
//! solana-system-interface
use {
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
    solana_sdk_ids::{system_program, sysvar},
};

/// Inlined `SystemInstruction::AdvanceNonceAccount` instruction data to avoid
/// solana_system_interface and bincode deps
const ADVANCE_NONCE_DATA: [u8; 4] = [4, 0, 0, 0];

/// Check if the given instruction data is the same as
/// `SystemInstruction::AdvanceNonceAccount`.
///
/// NOTE: It's possible for additional data to exist after the 4th byte, but
/// users of this function only look at the first 4 bytes.
pub fn is_advance_nonce_instruction_data(data: &[u8]) -> bool {
    data.get(0..4) == Some(&ADVANCE_NONCE_DATA)
}

/// Inlined `advance_nonce_account` instruction creator to avoid
/// solana_system_interface and bincode deps
pub(crate) fn advance_nonce_account_instruction(
    nonce_pubkey: &Address,
    nonce_authority_pubkey: &Address,
) -> Instruction {
    Instruction::new_with_bytes(
        system_program::id(),
        &ADVANCE_NONCE_DATA,
        vec![
            AccountMeta::new(*nonce_pubkey, false),
            #[allow(deprecated)]
            AccountMeta::new_readonly(sysvar::recent_blockhashes::id(), false),
            AccountMeta::new_readonly(*nonce_authority_pubkey, true),
        ],
    )
}

#[cfg(test)]
mod test {
    use {
        super::*,
        solana_system_interface::instruction::{advance_nonce_account, SystemInstruction},
    };

    #[test]
    fn inline_instruction_data_matches_program() {
        let nonce = Address::new_unique();
        let nonce_authority = Address::new_unique();
        assert_eq!(
            advance_nonce_account_instruction(&nonce, &nonce_authority),
            advance_nonce_account(&nonce, &nonce_authority),
        );
    }

    #[test]
    fn test_advance_nonce_ix_prefix() {
        let advance_nonce_ix: SystemInstruction =
            bincode::deserialize(&ADVANCE_NONCE_DATA).unwrap();
        assert_eq!(advance_nonce_ix, SystemInstruction::AdvanceNonceAccount);
    }
}
