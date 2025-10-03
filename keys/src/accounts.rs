//! Instruction <-> Transaction account compilation.

use {
    crate::keys::KeyMap,
    mollusk_svm_error::error::MolluskError,
    solana_account::{Account, AccountSharedData},
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
    solana_transaction_context::{IndexOfAccount, InstructionAccount, TransactionAccount},
};

// Helper struct to avoid cloning instruction data.
pub struct CompiledInstructionWithoutData {
    pub program_id_index: u8,
    pub accounts: Vec<u8>,
}

pub fn compile_instruction_without_data(
    key_map: &KeyMap,
    instruction: &Instruction,
) -> CompiledInstructionWithoutData {
    CompiledInstructionWithoutData {
        program_id_index: key_map.position(&instruction.program_id).unwrap() as u8,
        accounts: instruction
            .accounts
            .iter()
            .map(|account_meta| key_map.position(&account_meta.pubkey).unwrap() as u8)
            .collect(),
    }
}

pub fn compile_instruction_accounts(
    key_map: &KeyMap,
    compiled_instruction: &CompiledInstructionWithoutData,
) -> Vec<InstructionAccount> {
    compiled_instruction
        .accounts
        .iter()
        .map(|&index_in_transaction| {
            let index_in_transaction = index_in_transaction as usize;
            InstructionAccount::new(
                index_in_transaction as IndexOfAccount,
                key_map.is_signer_at_index(index_in_transaction),
                key_map.is_writable_at_index(index_in_transaction),
            )
        })
        .collect()
}

pub fn compile_transaction_accounts_for_instruction(
    key_map: &KeyMap,
    instruction: &Instruction,
    accounts: &[(Pubkey, Account)],
    stub_out_program_account: Option<Box<dyn Fn() -> Account>>,
) -> Vec<TransactionAccount> {
    key_map
        .keys()
        .map(|key| {
            let account = accounts
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, account)| AccountSharedData::from(account.clone()));

            if let Some(account) = account {
                (*key, account)
            } else if let Some(stub_out_program_account) = &stub_out_program_account {
                if instruction.program_id == *key {
                    (*key, stub_out_program_account().into())
                } else {
                    panic!("{}", MolluskError::AccountMissing(key))
                }
            } else {
                panic!("{}", MolluskError::AccountMissing(key))
            }
        })
        .collect()
}

pub fn compile_transaction_accounts(
    key_map: &KeyMap,
    instructions: &[Instruction],
    accounts: &[(Pubkey, Account)],
    stub_out_program_account: Option<Box<dyn Fn() -> Account>>,
) -> Vec<TransactionAccount> {
    key_map
        .keys()
        .map(|key| {
            let account = accounts
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, account)| AccountSharedData::from(account.clone()));

            if let Some(account) = account {
                (*key, account)
            } else if let Some(stub_out_program_account) = &stub_out_program_account {
                if instructions.iter().any(|ix| ix.program_id == *key) {
                    (*key, stub_out_program_account().into())
                } else {
                    panic!("{}", MolluskError::AccountMissing(key))
                }
            } else {
                panic!("{}", MolluskError::AccountMissing(key))
            }
        })
        .collect()
}
