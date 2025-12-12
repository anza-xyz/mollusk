//! Instruction <-> Transaction account compilation.

use {
    crate::keys::KeyMap,
    mollusk_svm_error::error::{MolluskError, MolluskPanic},
    solana_account::{Account, AccountSharedData},
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
    solana_transaction_context::{IndexOfAccount, InstructionAccount},
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
    let program_id_index = key_map
        .position(&instruction.program_id)
        .or_panic_with(MolluskError::ProgramIdNotMapped(&instruction.program_id));

    let program_id_index = u8::try_from(program_id_index)
        .or_panic_with(MolluskError::AccountIndexOverflow(program_id_index));

    let accounts: Vec<u8> = instruction
        .accounts
        .iter()
        .map(|account_meta| {
            let index = key_map
                .position(&account_meta.pubkey)
                .or_panic_with(MolluskError::AccountMissing(&account_meta.pubkey));

            u8::try_from(index).or_panic_with(MolluskError::AccountIndexOverflow(index))
        })
        .collect();

    CompiledInstructionWithoutData {
        program_id_index,
        accounts,
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

pub fn compile_transaction_accounts<'a>(
    key_map: &KeyMap,
    accounts: impl Iterator<Item = &'a (Pubkey, Account)>,
    fallback_accounts: &HashMap<Pubkey, Account>,
) -> Vec<(Pubkey, AccountSharedData)> {
    let accounts: Vec<_> = accounts.collect();
    key_map
        .keys()
        .map(|key| {
            let account = accounts
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, a)| AccountSharedData::from(a.clone()))
                .or_else(|| fallback_accounts.get(key).cloned().map(Into::into))
                .or_panic_with(MolluskError::AccountMissing(key));
            (*key, account)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use {super::*, solana_account::ReadableAccount, solana_instruction::AccountMeta};

    fn test_instruction(program_id: Pubkey, account_keys: &[Pubkey]) -> Instruction {
        Instruction::new_with_bytes(
            program_id,
            &[],
            account_keys
                .iter()
                .map(|pk| AccountMeta::new(*pk, false))
                .collect(),
        )
    }

    #[test]
    fn test_compile_instruction_without_data() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();
        let account2 = Pubkey::new_unique();

        let instruction = test_instruction(program_id, &[account1, account2]);
        let key_map = KeyMap::compile_from_instruction(&instruction);

        let compiled = compile_instruction_without_data(&key_map, &instruction);

        assert_eq!(
            compiled.program_id_index,
            key_map.position(&program_id).unwrap() as u8
        );
        assert_eq!(compiled.accounts.len(), 2);
        assert_eq!(
            compiled.accounts[0],
            key_map.position(&account1).unwrap() as u8
        );
        assert_eq!(
            compiled.accounts[1],
            key_map.position(&account2).unwrap() as u8
        );
    }

    #[test]
    fn test_compile_instruction_accounts() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();
        let account2 = Pubkey::new_unique();

        let instruction = Instruction::new_with_bytes(
            program_id,
            &[],
            vec![
                AccountMeta::new(account1, true),           // signer, writable
                AccountMeta::new_readonly(account2, false), // not signer, not writable
            ],
        );
        let key_map = KeyMap::compile_from_instruction(&instruction);
        let compiled_ix = compile_instruction_without_data(&key_map, &instruction);

        let instruction_accounts = compile_instruction_accounts(&key_map, &compiled_ix);

        assert_eq!(instruction_accounts.len(), 2);
        assert!(instruction_accounts[0].is_signer());
        assert!(instruction_accounts[0].is_writable());
        assert!(!instruction_accounts[1].is_signer());
        assert!(!instruction_accounts[1].is_writable());
    }

    #[test]
    fn test_compile_transaction_accounts_for_instruction_basic() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();
        let account2 = Pubkey::new_unique();

        let instruction = test_instruction(program_id, &[account1, account2]);
        let key_map = KeyMap::compile_from_instruction(&instruction);

        let accounts = [
            (program_id, Account::new(1000, 0, &Pubkey::default())),
            (account1, Account::new(100, 10, &Pubkey::default())),
            (account2, Account::new(200, 20, &Pubkey::default())),
        ];

        let fallbacks = HashMap::new();

        let result = compile_transaction_accounts(&key_map, accounts.iter(), &fallbacks);

        assert_eq!(result.len(), 3);
        // Verify accounts are present (order depends on KeyMap).
        assert!(result.iter().any(|(pk, _)| pk == &program_id));
        assert!(result.iter().any(|(pk, _)| pk == &account1));
        assert!(result.iter().any(|(pk, _)| pk == &account2));
    }

    #[test]
    fn test_compile_transaction_accounts_for_instruction_with_stub() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();

        let instruction = test_instruction(program_id, &[account1]);
        let key_map = KeyMap::compile_from_instruction(&instruction);

        // Only provide account1, not program_id.
        let accounts = [(account1, Account::new(100, 10, &Pubkey::default()))];

        let fallbacks = [(program_id, Account::new(999, 0, &Pubkey::default()))]
            .into_iter()
            .collect();

        let result = compile_transaction_accounts(&key_map, accounts.iter(), &fallbacks);

        assert_eq!(result.len(), 2);
        // Program account should have 999 lamports from stub.
        let program_account = result.iter().find(|(pk, _)| pk == &program_id).unwrap();
        assert_eq!(program_account.1.lamports(), 999);
    }

    #[test]
    fn test_compile_transaction_accounts_for_instruction_with_fallback() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();
        let fallback_key = Pubkey::new_unique();

        let instruction = test_instruction(program_id, &[account1, fallback_key]);
        let key_map = KeyMap::compile_from_instruction(&instruction);

        // Only provide program_id and account1, not fallback_key.
        let accounts = [
            (program_id, Account::new(1000, 0, &Pubkey::default())),
            (account1, Account::new(100, 10, &Pubkey::default())),
        ];

        let fallbacks = [(fallback_key, Account::new(555, 5, &Pubkey::default()))]
            .into_iter()
            .collect();

        let result = compile_transaction_accounts(&key_map, accounts.iter(), &fallbacks);

        assert_eq!(result.len(), 3);
        // Fallback account should have 555 lamports.
        let fb_account = result.iter().find(|(pk, _)| pk == &fallback_key).unwrap();
        assert_eq!(fb_account.1.lamports(), 555);
    }

    #[test]
    fn test_compile_transaction_accounts_basic() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();

        let instructions = [test_instruction(program_id, &[account1])];
        let key_map = KeyMap::compile_from_instructions(instructions.iter());

        let accounts = [
            (program_id, Account::new(1000, 0, &Pubkey::default())),
            (account1, Account::new(100, 10, &Pubkey::default())),
        ];

        let fallbacks = HashMap::new();

        let result = compile_transaction_accounts(&key_map, accounts.iter(), &fallbacks);

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|(pk, _)| pk == &program_id));
        assert!(result.iter().any(|(pk, _)| pk == &account1));
    }

    #[test]
    fn test_compile_transaction_accounts_with_fallback() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();
        let fallback_key = Pubkey::new_unique();

        let instructions = [test_instruction(program_id, &[account1, fallback_key])];
        let key_map = KeyMap::compile_from_instructions(instructions.iter());

        // Only provide program_id and account1.
        let accounts = [
            (program_id, Account::new(1000, 0, &Pubkey::default())),
            (account1, Account::new(100, 10, &Pubkey::default())),
        ];

        let fallbacks = [(fallback_key, Account::new(777, 7, &Pubkey::default()))]
            .into_iter()
            .collect();

        let result = compile_transaction_accounts(&key_map, accounts.iter(), &fallbacks);

        assert_eq!(result.len(), 3);
        let fb_account = result.iter().find(|(pk, _)| pk == &fallback_key).unwrap();
        assert_eq!(fb_account.1.lamports(), 777);
    }

    #[test]
    fn test_fallback_not_called_when_account_present() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();

        let instruction = test_instruction(program_id, &[account1]);
        let key_map = KeyMap::compile_from_instruction(&instruction);

        let accounts = [
            (program_id, Account::new(1000, 0, &Pubkey::default())),
            (account1, Account::new(100, 10, &Pubkey::default())),
        ];

        let fallbacks = [(account1, Account::new(999, 99, &Pubkey::default()))]
            .into_iter()
            .collect();

        let result = compile_transaction_accounts(&key_map, accounts.iter(), &fallbacks);

        // account1 should have original 100 lamports, not 999 from fallback.
        let acc = result.iter().find(|(pk, _)| pk == &account1).unwrap();
        assert_eq!(acc.1.lamports(), 100);
    }

    #[test]
    fn test_compile_instruction_without_data_deterministic() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();
        let account2 = Pubkey::new_unique();
        let account3 = Pubkey::new_unique();

        let instruction = test_instruction(program_id, &[account1, account2, account3]);

        let key_map1 = KeyMap::compile_from_instruction(&instruction);
        let compiled1 = compile_instruction_without_data(&key_map1, &instruction);

        let key_map2 = KeyMap::compile_from_instruction(&instruction);
        let compiled2 = compile_instruction_without_data(&key_map2, &instruction);

        assert_eq!(compiled1.program_id_index, compiled2.program_id_index);
        assert_eq!(compiled1.accounts, compiled2.accounts);
    }

    #[test]
    #[should_panic(expected = "Account index exceeds maximum of 255")]
    fn test_compile_instruction_without_data_account_index_overflow() {
        let mut key_map = KeyMap::default();

        for _ in 0..256 {
            let pubkey = Pubkey::new_unique();
            key_map.add_program(pubkey);
        }

        let program_id = Pubkey::new_unique();
        key_map.add_program(program_id);

        let instruction = Instruction::new_with_bytes(program_id, &[], vec![]);

        let _ = compile_instruction_without_data(&key_map, &instruction);
    }

    #[test]
    #[should_panic(expected = "Program ID required by the instruction is not mapped")]
    fn test_compile_instruction_without_data_missing_program_id() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();
        let instruction = test_instruction(program_id, &[account1]);

        let mut key_map = KeyMap::default();
        key_map.add_account(&AccountMeta::new(account1, false));

        let _ = compile_instruction_without_data(&key_map, &instruction);
    }

    #[test]
    #[should_panic(expected = "An account required by the instruction was not provided")]
    fn test_compile_instruction_without_data_missing_account() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();
        let account_missing = Pubkey::new_unique();
        let instruction = Instruction::new_with_bytes(
            program_id,
            &[],
            vec![
                AccountMeta::new(account1, false),
                AccountMeta::new(account_missing, false),
            ],
        );

        let mut key_map = KeyMap::default();
        key_map.add_program(program_id);
        key_map.add_account(&AccountMeta::new(account1, false));

        let _ = compile_instruction_without_data(&key_map, &instruction);
    }
}
