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
    instructions: &[Instruction],
    accounts: impl Iterator<Item = &'a (Pubkey, Account)>,
    get_loader_key: impl Fn(&Pubkey) -> Pubkey,
) -> (SanitizedMessage, Vec<(Pubkey, AccountSharedData)>) {
    compile_accounts_with_payer(instructions, accounts, get_loader_key, None)
}

pub fn compile_accounts_with_payer<'a>(
    instructions: &[Instruction],
    accounts: impl Iterator<Item = &'a (Pubkey, Account)>,
    get_loader_key: impl Fn(&Pubkey) -> Pubkey,
    payer: Option<&Pubkey>,
) -> (SanitizedMessage, Vec<(Pubkey, AccountSharedData)>) {
    let message = Message::new(instructions, payer);
    let sanitized_message = SanitizedMessage::Legacy(LegacyMessage::new(message, &HashSet::new()));

    let accounts: Vec<_> = accounts.collect();

    let fallback_accounts =
        get_account_fallbacks(&sanitized_message, &accounts, get_loader_key, payer);

    let transaction_accounts = build_transaction_accounts(
        &sanitized_message,
        &accounts,
        instructions,
        &fallback_accounts,
    );

    (sanitized_message, transaction_accounts)
}

// Determine the accounts to fallback to during account compilation.
fn get_account_fallbacks(
    message: &SanitizedMessage,
    accounts: &[&(Pubkey, Account)],
    get_loader_key: impl Fn(&Pubkey) -> Pubkey,
    payer: Option<&Pubkey>,
) -> HashMap<Pubkey, Account> {
    // Use a HashSet for fast lookups.
    let account_keys: HashSet<&Pubkey> = accounts.iter().map(|(key, _)| key).collect();

    let mut fallbacks = HashMap::new();

    // Top-level target programs.
    message
        .program_instructions_iter()
        .for_each(|(program_id, _)| {
            if !account_keys.contains(program_id) {
                // Fallback to a stub.
                fallbacks.insert(
                    *program_id,
                    Account {
                        owner: get_loader_key(program_id),
                        executable: true,
                        ..Default::default()
                    },
                );
            }
        });

    // Transaction fee payer.
    if let Some(payer) = payer.filter(|payer| !account_keys.contains(payer)) {
        fallbacks.entry(*payer).or_default();
    }

    fallbacks
}

fn build_transaction_accounts(
    message: &SanitizedMessage,
    accounts: &[&(Pubkey, Account)],
    all_instructions: &[Instruction],
    fallback_accounts: &HashMap<Pubkey, Account>,
) -> Vec<(Pubkey, AccountSharedData)> {
    let program_ids: HashSet<Pubkey> = all_instructions.iter().map(|ix| ix.program_id).collect();

    message
        .account_keys()
        .iter()
        .map(|key| {
            if program_ids.contains(key) {
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
                // We will never have a fallback for the Instructions sysvar.
                let (_, account) = crate::instructions_sysvar::keyed_account(message);
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
