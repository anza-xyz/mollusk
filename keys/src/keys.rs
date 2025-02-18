//! Instruction <-> Transaction key deduplication and privilege handling.
//!
//! Solana instructions and transactions are designed to be intentionally
//! verbosely declarative, to provide the runtime with granular directives
//! for manipulating chain state.
//!
//! As a result, when a transaction is _compiled_, many steps occur:
//! * Ensuring there is a fee payer.
//! * Ensuring there is a signature.
//! * Deduplicating account keys.
//! * Configuring the highest role awarded to each account key.
//! * ...
//!
//! This modules provides utilities for deduplicating account keys and
//! handling the highest role awarded to each account key. It can be used
//! standalone or within the other transaction helpers in this library to build
//! custom transactions for the SVM API with the required structure and roles.
//!
//! This implementation closely follows the implementation in the Anza SDK
//! for `Message::new_with_blockhash`. For more information, see:
//! <https://github.com/anza-xyz/agave/blob/c6e8239843af8e6301cd198e39d0a44add427bef/sdk/program/src/message/legacy.rs#L357>.

use {
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
    std::collections::{HashMap, HashSet},
};

/// Wrapper around a hashmap of account keys and their corresponding roles
/// (`is_signer`, `is_writable`).
///
/// On compilation, keys are awarded the highest role they are assigned in the
/// transaction, and the hash map provides deduplication.
///
/// The map can be queried by key for `is_signer` and `is_writable` roles.
#[derive(Debug, Default)]
pub struct KeyMap {
    map: HashMap<Pubkey, (bool, bool)>,
    program_ids: HashSet<Pubkey>,
}

impl KeyMap {
    /// Add a single account meta to the key map.
    pub fn add_account(&mut self, meta: &AccountMeta) {
        let entry = self.map.entry(meta.pubkey).or_default();
        entry.0 |= meta.is_signer;
        entry.1 |= meta.is_writable;
    }

    /// Add a list of account metas to the key map.
    pub fn add_accounts<'a>(&mut self, accounts: impl Iterator<Item = &'a AccountMeta>) {
        for meta in accounts {
            self.add_account(meta);
        }
    }

    /// Add keys from a single instruction to the key map.
    pub fn add_instruction(&mut self, instruction: &Instruction) {
        self.add_program(instruction.program_id);
        self.add_accounts(instruction.accounts.iter());
    }

    /// Add keys from multiple instructions to the key map.
    pub fn add_instructions<'a>(&mut self, instructions: impl Iterator<Item = &'a Instruction>) {
        for instruction in instructions {
            self.add_instruction(instruction);
        }
    }

    /// Add a single program ID to the key map.
    pub fn add_program(&mut self, program_id: Pubkey) {
        self.map.insert(program_id, (false, false));
        self.program_ids.insert(program_id);
    }

    /// Add a list of program IDs to the key map.
    pub fn add_programs<'a>(&mut self, program_ids: impl Iterator<Item = &'a Pubkey>) {
        for program_id in program_ids {
            self.add_program(*program_id);
        }
    }

    /// Compile a new key map with the provided program IDs and accounts.
    pub fn compile<'a>(
        program_ids: impl Iterator<Item = &'a Pubkey>,
        accounts: impl Iterator<Item = &'a AccountMeta>,
    ) -> Self {
        let mut map = Self::default();
        map.add_programs(program_ids);
        map.add_accounts(accounts);
        map
    }

    pub fn compile_from_instruction(instruction: &Instruction) -> Self {
        let mut map = Self::default();
        map.add_instruction(instruction);
        map
    }

    /// Compile a new key map with the keys from multiple provided instructions.
    pub fn compile_from_instructions<'a>(
        instructions: impl Iterator<Item = &'a Instruction>,
    ) -> Self {
        let mut map = Self::default();
        map.add_instructions(instructions);
        map
    }

    /// Query the key map for the `is_invoked` role of a key.
    ///
    /// This role is only for program IDs designated in an instruction.
    pub fn is_invoked(&self, key: &Pubkey) -> bool {
        self.program_ids.contains(key)
    }

    /// Query the key map for the `is_invoked` role of a key at the specified
    /// index.
    ///
    /// This role is only for program IDs designated in an instruction.
    pub fn is_invoked_at_index(&self, index: usize) -> bool {
        self.map
            .iter()
            .nth(index)
            .map(|(k, _)| self.program_ids.contains(k))
            .unwrap_or(false)
    }

    /// Query the key map for the `is_signer` role of a key.
    pub fn is_signer(&self, key: &Pubkey) -> bool {
        self.map.get(key).map(|(s, _)| *s).unwrap_or(false)
    }

    /// Query the key map for the `is_signer` role of a key at the specified
    /// index.
    pub fn is_signer_at_index(&self, index: usize) -> bool {
        self.map
            .values()
            .nth(index)
            .map(|(s, _)| *s)
            .unwrap_or(false)
    }

    /// Get the number of keys in the key map that have the `is_signer` role.
    pub fn is_signer_count(&self) -> usize {
        self.map.values().filter(|(s, _)| *s).count()
    }

    /// Query the key map for the `is_writable` role of a key.
    pub fn is_writable(&self, key: &Pubkey) -> bool {
        self.map.get(key).map(|(_, w)| *w).unwrap_or(false)
    }

    /// Query the key map for the `is_writable` role of a key at the specified
    /// index.
    pub fn is_writable_at_index(&self, index: usize) -> bool {
        self.map
            .values()
            .nth(index)
            .map(|(_, w)| *w)
            .unwrap_or(false)
    }

    /// Get the number of keys in the key map that have the `is_writable` role.
    pub fn is_writable_count(&self) -> usize {
        self.map.values().filter(|(_, w)| *w).count()
    }

    /// Get the key at the specified index in the key map.
    pub fn key_at_index(&self, index: usize) -> Option<&Pubkey> {
        self.map.keys().nth(index)
    }

    /// Get the keys in the key map.
    pub fn keys(&self) -> impl Iterator<Item = &Pubkey> {
        self.map.keys()
    }

    /// Get the position of a key in the key map.
    ///
    /// This returns its position in the hash map's keys iterator.
    pub fn position(&self, key: &Pubkey) -> Option<usize> {
        self.map.keys().position(|k| k == key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile() {
        let program_id1 = Pubkey::new_unique();
        let program_id2 = Pubkey::new_unique();

        let key1 = Pubkey::new_unique();
        let key2 = Pubkey::new_unique();
        let key3 = Pubkey::new_unique();

        let metas1 = &[
            // Key1: writable
            AccountMeta::new(key1, false),
            // Key2: signer
            AccountMeta::new_readonly(key2, true),
        ];
        let metas2 = &[
            // Key1: signer
            AccountMeta::new_readonly(key1, true),
            // Key2: signer
            AccountMeta::new_readonly(key2, true),
        ];
        let metas3 = &[
            // Key2: readonly
            AccountMeta::new_readonly(key2, false),
            // Key3: readonly
            AccountMeta::new_readonly(key3, false),
        ];

        let run_checks = |key_map: &KeyMap| {
            // Expected roles:
            // Key1: signer, writable
            // Key2: signer
            // Key3: readonly
            assert!(key_map.is_signer(&key1));
            assert!(key_map.is_writable(&key1));
            assert!(key_map.is_signer(&key2));
            assert!(!key_map.is_writable(&key2));
            assert!(!key_map.is_signer(&key3));
            assert!(!key_map.is_writable(&key3));

            // Try with positional arguments.
            let key1_pos = key_map.position(&key1).unwrap();
            let key2_pos = key_map.position(&key2).unwrap();
            let key3_pos = key_map.position(&key3).unwrap();
            assert!(key_map.is_signer_at_index(key1_pos));
            assert!(key_map.is_writable_at_index(key1_pos));
            assert!(key_map.is_signer_at_index(key2_pos));
            assert!(!key_map.is_writable_at_index(key2_pos));
            assert!(!key_map.is_signer_at_index(key3_pos));
            assert!(!key_map.is_writable_at_index(key3_pos));

            // Also double-check index-pubkey compatibility.
            assert_eq!(key_map.key_at_index(key1_pos).unwrap(), &key1);
            assert_eq!(key_map.key_at_index(key2_pos).unwrap(), &key2);
            assert_eq!(key_map.key_at_index(key3_pos).unwrap(), &key3);
            let program_id1_pos = key_map.position(&program_id1).unwrap();
            let program_id2_pos = key_map.position(&program_id2).unwrap();
            assert_eq!(key_map.key_at_index(program_id1_pos).unwrap(), &program_id1);
            assert_eq!(key_map.key_at_index(program_id2_pos).unwrap(), &program_id2);
        };

        // With manual adds.
        let mut key_map = KeyMap::default();
        key_map.add_programs([program_id1, program_id2].iter());
        key_map.add_accounts(metas1.iter());
        key_map.add_accounts(metas2.iter());
        key_map.add_accounts(metas3.iter());
        run_checks(&key_map);

        // With `compile`.
        let key_map = KeyMap::compile(
            [program_id1, program_id2].iter(),
            metas1.iter().chain(metas2.iter()).chain(metas3.iter()),
        );
        run_checks(&key_map);
    }
}
