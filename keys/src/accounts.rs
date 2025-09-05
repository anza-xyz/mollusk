//! Instruction <-> Transaction account compilation.

use {
    crate::keys::KeyMap,
    mollusk_svm_error::error::{MolluskError, MolluskPanic},
    solana_account::{Account, AccountSharedData},
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
    solana_transaction_context::{IndexOfAccount, InstructionAccount, TransactionAccount},
    std::collections::HashMap,
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

pub fn compile_transaction_accounts_for_instruction<'a>(
    key_map: &KeyMap,
    instruction: &Instruction,
    accounts: impl Iterator<Item = &'a (Pubkey, Account)>,
    stub_out_program_account: Option<Box<dyn Fn() -> Account>>,
) -> Vec<TransactionAccount> {
    let len = key_map.len();
    let mut by_key: HashMap<Pubkey, AccountSharedData> = HashMap::with_capacity(len);

    for (key, account) in accounts {
        if key_map.contains_key(key) {
            by_key.insert(*key, AccountSharedData::from(account.clone()));
        }
    }

    key_map
        .keys()
        .map(|key| {
            if let Some(stub_out_program_account) = &stub_out_program_account {
                if instruction.program_id == *key {
                    return (*key, stub_out_program_account().into());
                }
            }
            let account = by_key
                .remove(key)
                .or_panic_with(MolluskError::AccountMissing(key));
            (*key, account)
        })
        .collect()
}

pub fn compile_transaction_accounts<'a>(
    key_map: &KeyMap,
    instructions: &[Instruction],
    accounts: impl Iterator<Item = &'a (Pubkey, Account)>,
    stub_out_program_account: Option<Box<dyn Fn() -> Account>>,
) -> Vec<TransactionAccount> {
    let len = key_map.len();
    let mut by_key: HashMap<Pubkey, AccountSharedData> = HashMap::with_capacity(len);

    for (key, account) in accounts {
        if key_map.contains_key(key) {
            by_key.insert(*key, AccountSharedData::from(account.clone()));
        }
    }

    key_map
        .keys()
        .map(|key| {
            if let Some(stub_out_program_account) = &stub_out_program_account {
                if instructions.iter().any(|ix| ix.program_id == *key) {
                    return (*key, stub_out_program_account().into());
                }
            }
            let account = by_key
                .remove(key)
                .or_panic_with(MolluskError::AccountMissing(key));
            (*key, account)
        })
        .collect()
}
