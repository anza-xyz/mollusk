//! Instruction <-> Transaction account compilation, with key deduplication,
//! privilege handling, and program account stubbing.

use {
    crate::InstructionAccountPrivilegeOverrides,
    mollusk_svm_error::error::{MolluskError, MolluskPanic},
    solana_account::{Account, AccountSharedData, WritableAccount},
    solana_instruction::Instruction,
    solana_message::{
        compiled_instruction::CompiledInstruction, Address, Hash, LegacyMessage, Message,
        MessageHeader, SanitizedMessage,
    },
    solana_pubkey::Pubkey,
    std::collections::{BTreeMap, HashMap, HashSet},
};

// Static empty HashSet for message sanitization — avoids allocation per call.
static EMPTY_HASHSET: std::sync::LazyLock<HashSet<Pubkey>> = std::sync::LazyLock::new(HashSet::new);

pub fn compile_accounts(
    instructions: &[Instruction],
    accounts: &[(Pubkey, Account)],
    fallback_accounts: &HashMap<Pubkey, Account>,
    privilege_overrides: InstructionAccountPrivilegeOverrides,
) -> (SanitizedMessage, Vec<(Pubkey, AccountSharedData)>) {
    let message = compile_message(instructions, privilege_overrides);
    let sanitized_message = SanitizedMessage::Legacy(LegacyMessage::new(message, &EMPTY_HASHSET));

    let transaction_accounts = build_transaction_accounts(
        &sanitized_message,
        accounts,
        instructions,
        fallback_accounts,
    );

    (sanitized_message, transaction_accounts)
}

fn compile_message(
    instructions: &[Instruction],
    privilege_overrides: InstructionAccountPrivilegeOverrides,
) -> Message {
    if privilege_overrides == InstructionAccountPrivilegeOverrides::default() {
        return Message::new(instructions, None);
    }

    let compiled_keys = CompiledKeys::compile(instructions, privilege_overrides);
    let (header, account_keys) = compiled_keys
        .try_into_message_components()
        .expect("overflow when compiling message keys");
    let instructions = compile_instructions(instructions, &account_keys);

    Message::new_with_compiled_instructions(
        header.num_required_signatures,
        header.num_readonly_signed_accounts,
        header.num_readonly_unsigned_accounts,
        account_keys,
        Hash::default(),
        instructions,
    )
}

fn compile_instructions(ixs: &[Instruction], keys: &[Address]) -> Vec<CompiledInstruction> {
    ixs.iter()
        .map(|ix| CompiledInstruction {
            program_id_index: position(keys, &ix.program_id.into()),
            data: ix.data.clone(),
            accounts: ix
                .accounts
                .iter()
                .map(|account_meta| position(keys, &account_meta.pubkey.into()))
                .collect(),
        })
        .collect()
}

fn position(keys: &[Address], key: &Address) -> u8 {
    keys.iter()
        .position(|candidate| candidate == key)
        .and_then(|index| u8::try_from(index).ok())
        .expect("compiled message missing instruction key")
}

#[derive(Default)]
struct CompiledKeys {
    key_meta_map: BTreeMap<Address, CompiledKeyMeta>,
}

#[derive(Default)]
struct CompiledKeyMeta {
    is_signer: bool,
    is_writable: bool,
}

impl CompiledKeys {
    fn compile(
        instructions: &[Instruction],
        privilege_overrides: InstructionAccountPrivilegeOverrides,
    ) -> Self {
        let mut key_meta_map = BTreeMap::<Address, CompiledKeyMeta>::new();

        for ix in instructions {
            key_meta_map.entry(ix.program_id.into()).or_default();

            for account_meta in &ix.accounts {
                let meta = key_meta_map.entry(account_meta.pubkey.into()).or_default();
                meta.is_signer |= account_meta.is_signer || privilege_overrides.force_signer;
                meta.is_writable |= account_meta.is_writable || privilege_overrides.force_writable;
            }
        }

        Self { key_meta_map }
    }

    fn try_into_message_components(self) -> Result<(MessageHeader, Vec<Address>), &'static str> {
        let try_into_u8 = |num: usize| {
            u8::try_from(num).map_err(|_| "account index overflowed during compilation")
        };

        let writable_signer_keys: Vec<Address> = self
            .key_meta_map
            .iter()
            .filter_map(|(key, meta)| (meta.is_signer && meta.is_writable).then_some(*key))
            .collect();
        let readonly_signer_keys: Vec<Address> = self
            .key_meta_map
            .iter()
            .filter_map(|(key, meta)| (meta.is_signer && !meta.is_writable).then_some(*key))
            .collect();
        let writable_non_signer_keys: Vec<Address> = self
            .key_meta_map
            .iter()
            .filter_map(|(key, meta)| (!meta.is_signer && meta.is_writable).then_some(*key))
            .collect();
        let readonly_non_signer_keys: Vec<Address> = self
            .key_meta_map
            .iter()
            .filter_map(|(key, meta)| (!meta.is_signer && !meta.is_writable).then_some(*key))
            .collect();

        let signers_len = writable_signer_keys
            .len()
            .saturating_add(readonly_signer_keys.len());

        let header = MessageHeader {
            num_required_signatures: try_into_u8(signers_len)?,
            num_readonly_signed_accounts: try_into_u8(readonly_signer_keys.len())?,
            num_readonly_unsigned_accounts: try_into_u8(readonly_non_signer_keys.len())?,
        };

        let account_keys = std::iter::empty()
            .chain(writable_signer_keys)
            .chain(readonly_signer_keys)
            .chain(writable_non_signer_keys)
            .chain(readonly_non_signer_keys)
            .collect();

        Ok((header, account_keys))
    }
}

fn build_transaction_accounts(
    message: &SanitizedMessage,
    accounts: &[(Pubkey, Account)],
    all_instructions: &[Instruction],
    fallback_accounts: &HashMap<Pubkey, Account>,
) -> Vec<(Pubkey, AccountSharedData)> {
    let program_ids: HashSet<Pubkey> = all_instructions.iter().map(|ix| ix.program_id).collect();

    // Pre-index accounts by pubkey for O(1) lookups instead of O(n) linear scans.
    let account_map: HashMap<&Pubkey, &Account> = accounts.iter().map(|(k, a)| (k, a)).collect();

    message
        .account_keys()
        .iter()
        .map(|key| {
            if program_ids.contains(key) {
                if let Some(provided_account) = account_map.get(key) {
                    return (*key, AccountSharedData::from((*provided_account).clone()));
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
                if let Some(provided_account) = account_map.get(key) {
                    return (*key, AccountSharedData::from((*provided_account).clone()));
                }
                if let Some(fallback) = fallback_accounts.get(key) {
                    return (*key, AccountSharedData::from(fallback.clone()));
                }
                let (_, account) =
                    crate::instructions_sysvar::keyed_account(all_instructions.iter());
                return (*key, account.into());
            }

            let account = account_map
                .get(key)
                .map(|a| AccountSharedData::from((*a).clone()))
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
