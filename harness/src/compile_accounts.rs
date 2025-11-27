//! Instruction <-> Transaction account compilation, with key deduplication,
//! privilege handling, and program account stubbing.

use {
    mollusk_svm_keys::{
        accounts::{
            compile_instruction_accounts, compile_instruction_without_data,
            compile_transaction_accounts_for_instruction,
        },
        keys::KeyMap,
    },
    solana_account::{Account, AccountSharedData, WritableAccount},
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
    solana_transaction_context::InstructionAccount,
};

pub struct CompiledAccounts {
    pub program_id_index: u16,
    pub instruction_accounts: Vec<InstructionAccount>,
    pub transaction_accounts: Vec<(Pubkey, AccountSharedData)>,
}

pub fn compile_accounts<'a, 'b, I>(
    instruction_index: usize,
    all_instructions: I,
    accounts: impl Iterator<Item = &'a (Pubkey, Account)>,
    loader_key: Pubkey,
) -> CompiledAccounts
where
    I: IntoIterator<Item = &'b Instruction>,
{
    // Collect instruction references for the instructions sysvar.
    let instruction_refs: Vec<&Instruction> = all_instructions.into_iter().collect();
    let instruction = instruction_refs[instruction_index];

    let stub_out_program_account = move || {
        let mut program_account = Account::default();
        program_account.set_owner(loader_key);
        program_account.set_executable(true);
        program_account
    };

    let fallback_to_instructions_sysvar = |pubkey: &Pubkey| -> Option<Account> {
        (pubkey == &solana_instructions_sysvar::ID)
            .then(|| crate::instructions_sysvar::keyed_account(instruction_refs.iter().copied()).1)
    };

    let key_map = KeyMap::compile_from_instruction(instruction);
    let compiled_instruction = compile_instruction_without_data(&key_map, instruction)
        .unwrap_or_else(|err| panic!("Failed to compile instruction: {}", err));
    let instruction_accounts = compile_instruction_accounts(&key_map, &compiled_instruction);
    let transaction_accounts = compile_transaction_accounts_for_instruction(
        &key_map,
        instruction,
        accounts,
        Some(stub_out_program_account),
        Some(fallback_to_instructions_sysvar),
    );

    CompiledAccounts {
        program_id_index: compiled_instruction.program_id_index as u16,
        instruction_accounts,
        transaction_accounts,
    }
}
