//! Instruction <-> Transaction account compilation, with key deduplication,
//! privilege handling, and program account stubbing.

use {
    mollusk_svm_error::error::{MolluskError, MolluskPanic},
    solana_account::{Account, AccountSharedData, WritableAccount},
    solana_instruction::Instruction,
    solana_message::{LegacyMessage, Message, SanitizedMessage},
    solana_pubkey::Pubkey,
    std::collections::{HashMap, HashSet},
};

pub fn compile_accounts<'a>(
    instruction: &Instruction,
    accounts: impl Iterator<Item = &'a (Pubkey, Account)>,
    fallback_accounts: &HashMap<Pubkey, Account>,
) -> (SanitizedMessage, Vec<(Pubkey, AccountSharedData)>) {
    let message = Message::new(std::slice::from_ref(instruction), None);
    let sanitized_message = SanitizedMessage::Legacy(LegacyMessage::new(message, &HashSet::new()));

    let accounts: Vec<_> = accounts.collect();
    let transaction_accounts = build_transaction_accounts(
        &sanitized_message,
        instruction.program_id,
        &accounts,
        std::slice::from_ref(instruction),
        fallback_accounts,
    );

    (sanitized_message, transaction_accounts)
}

fn build_transaction_accounts(
    message: &SanitizedMessage,
    program_id: Pubkey,
    accounts: &[&(Pubkey, Account)],
    all_instructions: &[Instruction],
    fallback_accounts: &HashMap<Pubkey, Account>,
) -> Vec<(Pubkey, AccountSharedData)> {
    message
        .account_keys()
        .iter()
        .map(|key| {
            if *key == program_id {
                if let Some(provided_account) = accounts.iter().find(|(k, _)| k == key) {
                    return (*key, AccountSharedData::from(provided_account.1.clone()));
                }
                if let Some(fallback) = fallback_accounts.get(key) {
                    return (*key, AccountSharedData::from(fallback.clone()));
                }
                // This shouldn't happen if fallbacks are set up correctly.
                let mut program_account = Account::default();
                program_account.set_executable(true);
                return (*key, program_account.into());
            }

            if *key == solana_instructions_sysvar::ID {
                if let Some((_, provided_account)) = accounts.iter().find(|(k, _)| k == key) {
                    return (*key, AccountSharedData::from(provided_account.clone()));
                }
                if let Some(fallback) = fallback_accounts.get(key) {
                    return (*key, AccountSharedData::from(fallback.clone()));
                }
                let (_, account) =
                    crate::instructions_sysvar::keyed_account(all_instructions.iter());
                return (*key, account.into());
            }

            let account = accounts
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, a)| AccountSharedData::from(a.clone()))
                .or_else(|| {
                    fallback_accounts
                        .get(key)
                        .map(|a| AccountSharedData::from(a.clone()))
                })
                .or_panic_with(MolluskError::AccountMissing(key));

            (*key, account)
        })
        .collect()
}
