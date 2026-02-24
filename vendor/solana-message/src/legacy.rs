//! The original and current Solana message format.
//!
//! This crate defines two versions of `Message` in their own modules:
//! [`legacy`] and [`v0`]. `legacy` is the current version as of Solana 1.10.0.
//! `v0` is a [future message format] that encodes more account keys into a
//! transaction than the legacy format.
//!
//! [`legacy`]: crate::legacy
//! [`v0`]: crate::v0
//! [future message format]: https://docs.solanalabs.com/proposals/versioned-transactions

#![allow(clippy::arithmetic_side_effects)]

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};
#[cfg(feature = "frozen-abi")]
use solana_frozen_abi_macro::{frozen_abi, AbiExample};
use {
    crate::{
        compiled_instruction::CompiledInstruction, compiled_keys::CompiledKeys,
        inline_nonce::advance_nonce_account_instruction, MessageHeader,
    },
    solana_address::Address,
    solana_hash::Hash,
    solana_instruction::Instruction,
    solana_sanitize::{Sanitize, SanitizeError},
    solana_sdk_ids::bpf_loader_upgradeable,
    std::{collections::HashSet, convert::TryFrom},
};

fn position(keys: &[Address], key: &Address) -> u8 {
    keys.iter().position(|k| k == key).unwrap() as u8
}

fn compile_instruction(ix: &Instruction, keys: &[Address]) -> CompiledInstruction {
    let accounts: Vec<_> = ix
        .accounts
        .iter()
        .map(|account_meta| position(keys, &account_meta.pubkey))
        .collect();

    CompiledInstruction {
        program_id_index: position(keys, &ix.program_id),
        data: ix.data.clone(),
        accounts,
    }
}

fn compile_instructions(ixs: &[Instruction], keys: &[Address]) -> Vec<CompiledInstruction> {
    ixs.iter().map(|ix| compile_instruction(ix, keys)).collect()
}

/// A Solana transaction message (legacy).
///
/// See the crate documentation for further description.
///
/// Some constructors accept an optional `payer`, the account responsible for
/// paying the cost of executing a transaction. In most cases, callers should
/// specify the payer explicitly in these constructors. In some cases though,
/// the caller is not _required_ to specify the payer, but is still allowed to:
/// in the `Message` structure, the first account is always the fee-payer, so if
/// the caller has knowledge that the first account of the constructed
/// transaction's `Message` is both a signer and the expected fee-payer, then
/// redundantly specifying the fee-payer is not strictly required.
// NOTE: Serialization-related changes must be paired with the custom serialization
// for versioned messages in the `RemainingLegacyMessage` struct.
#[cfg_attr(
    feature = "frozen-abi",
    frozen_abi(digest = "GXpvLNiMCnjnZpQEDKpc2NBpsqmRnAX7ZTCy9JmvG8Dg"),
    derive(AbiExample)
)]
#[cfg_attr(
    feature = "serde",
    derive(Deserialize, Serialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct Message {
    /// The message header, identifying signed and read-only `account_keys`.
    // NOTE: Serialization-related changes must be paired with the direct read at sigverify.
    pub header: MessageHeader,

    /// All the account keys used by this transaction.
    #[cfg_attr(feature = "serde", serde(with = "solana_short_vec"))]
    pub account_keys: Vec<Address>,

    /// The id of a recent ledger entry.
    pub recent_blockhash: Hash,

    /// Programs that will be executed in sequence and committed in one atomic transaction if all
    /// succeed.
    #[cfg_attr(feature = "serde", serde(with = "solana_short_vec"))]
    pub instructions: Vec<CompiledInstruction>,
}

impl Sanitize for Message {
    fn sanitize(&self) -> std::result::Result<(), SanitizeError> {
        // signing area and read-only non-signing area should not overlap
        if self.header.num_required_signatures as usize
            + self.header.num_readonly_unsigned_accounts as usize
            > self.account_keys.len()
        {
            return Err(SanitizeError::IndexOutOfBounds);
        }

        // there should be at least 1 RW fee-payer account.
        if self.header.num_readonly_signed_accounts >= self.header.num_required_signatures {
            return Err(SanitizeError::IndexOutOfBounds);
        }

        for ci in &self.instructions {
            if ci.program_id_index as usize >= self.account_keys.len() {
                return Err(SanitizeError::IndexOutOfBounds);
            }
            // A program cannot be a payer.
            if ci.program_id_index == 0 {
                return Err(SanitizeError::IndexOutOfBounds);
            }
            for ai in &ci.accounts {
                if *ai as usize >= self.account_keys.len() {
                    return Err(SanitizeError::IndexOutOfBounds);
                }
            }
        }
        self.account_keys.sanitize()?;
        self.recent_blockhash.sanitize()?;
        self.instructions.sanitize()?;
        Ok(())
    }
}

impl Message {
    /// Create a new `Message`.
    ///
    /// # Examples
    ///
    /// This example uses the [`solana_sdk`], [`solana_rpc_client`] and [`anyhow`] crates.
    ///
    /// [`solana_sdk`]: https://docs.rs/solana-sdk
    /// [`solana_rpc_client`]: https://docs.rs/solana-rpc-client
    /// [`anyhow`]: https://docs.rs/anyhow
    ///
    /// ```
    /// # use solana_example_mocks::{solana_keypair, solana_signer, solana_transaction};
    /// # use solana_example_mocks::solana_rpc_client;
    /// use anyhow::Result;
    /// use borsh::{BorshSerialize, BorshDeserialize};
    /// use solana_instruction::Instruction;
    /// use solana_keypair::Keypair;
    /// use solana_message::Message;
    /// use solana_address::Address;
    /// use solana_rpc_client::rpc_client::RpcClient;
    /// use solana_signer::Signer;
    /// use solana_transaction::Transaction;
    ///
    /// // A custom program instruction. This would typically be defined in
    /// // another crate so it can be shared between the on-chain program and
    /// // the client.
    /// #[derive(BorshSerialize, BorshDeserialize)]
    /// # #[borsh(crate = "borsh")]
    /// enum BankInstruction {
    ///     Initialize,
    ///     Deposit { lamports: u64 },
    ///     Withdraw { lamports: u64 },
    /// }
    ///
    /// fn send_initialize_tx(
    ///     client: &RpcClient,
    ///     program_id: Address,
    ///     payer: &Keypair
    /// ) -> Result<()> {
    ///
    ///     let bank_instruction = BankInstruction::Initialize;
    ///
    ///     let instruction = Instruction::new_with_borsh(
    ///         program_id,
    ///         &bank_instruction,
    ///         vec![],
    ///     );
    ///
    ///     let message = Message::new(
    ///         &[instruction],
    ///         Some(&payer.pubkey()),
    ///     );
    ///
    ///     let blockhash = client.get_latest_blockhash()?;
    ///     let mut tx = Transaction::new(&[payer], message, blockhash);
    ///     client.send_and_confirm_transaction(&tx)?;
    ///
    ///     Ok(())
    /// }
    /// #
    /// # let client = RpcClient::new(String::new());
    /// # let program_id = Address::new_unique();
    /// # let payer = Keypair::new();
    /// # send_initialize_tx(&client, program_id, &payer)?;
    /// #
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn new(instructions: &[Instruction], payer: Option<&Address>) -> Self {
        Self::new_with_blockhash(instructions, payer, &Hash::default())
    }

    /// Create a new message while setting the blockhash.
    ///
    /// # Examples
    ///
    /// This example uses the [`solana_sdk`], [`solana_rpc_client`] and [`anyhow`] crates.
    ///
    /// [`solana_sdk`]: https://docs.rs/solana-sdk
    /// [`solana_rpc_client`]: https://docs.rs/solana-rpc-client
    /// [`anyhow`]: https://docs.rs/anyhow
    ///
    /// ```
    /// # use solana_example_mocks::{solana_keypair, solana_signer, solana_transaction};
    /// # use solana_example_mocks::solana_rpc_client;
    /// use anyhow::Result;
    /// use borsh::{BorshSerialize, BorshDeserialize};
    /// use solana_instruction::Instruction;
    /// use solana_keypair::Keypair;
    /// use solana_message::Message;
    /// use solana_address::Address;
    /// use solana_rpc_client::rpc_client::RpcClient;
    /// use solana_signer::Signer;
    /// use solana_transaction::Transaction;
    ///
    /// // A custom program instruction. This would typically be defined in
    /// // another crate so it can be shared between the on-chain program and
    /// // the client.
    /// #[derive(BorshSerialize, BorshDeserialize)]
    /// # #[borsh(crate = "borsh")]
    /// enum BankInstruction {
    ///     Initialize,
    ///     Deposit { lamports: u64 },
    ///     Withdraw { lamports: u64 },
    /// }
    ///
    /// fn send_initialize_tx(
    ///     client: &RpcClient,
    ///     program_id: Address,
    ///     payer: &Keypair
    /// ) -> Result<()> {
    ///
    ///     let bank_instruction = BankInstruction::Initialize;
    ///
    ///     let instruction = Instruction::new_with_borsh(
    ///         program_id,
    ///         &bank_instruction,
    ///         vec![],
    ///     );
    ///
    ///     let blockhash = client.get_latest_blockhash()?;
    ///
    ///     let message = Message::new_with_blockhash(
    ///         &[instruction],
    ///         Some(&payer.pubkey()),
    ///         &blockhash,
    ///     );
    ///
    ///     let mut tx = Transaction::new_unsigned(message);
    ///     tx.sign(&[payer], tx.message.recent_blockhash);
    ///     client.send_and_confirm_transaction(&tx)?;
    ///
    ///     Ok(())
    /// }
    /// #
    /// # let client = RpcClient::new(String::new());
    /// # let program_id = Address::new_unique();
    /// # let payer = Keypair::new();
    /// # send_initialize_tx(&client, program_id, &payer)?;
    /// #
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn new_with_blockhash(
        instructions: &[Instruction],
        payer: Option<&Address>,
        blockhash: &Hash,
    ) -> Self {
        let compiled_keys = CompiledKeys::compile(instructions, payer.cloned());
        let (header, account_keys) = compiled_keys
            .try_into_message_components()
            .expect("overflow when compiling message keys");
        let instructions = compile_instructions(instructions, &account_keys);
        Self::new_with_compiled_instructions(
            header.num_required_signatures,
            header.num_readonly_signed_accounts,
            header.num_readonly_unsigned_accounts,
            account_keys,
            *blockhash,
            instructions,
        )
    }

    /// Create a new message for a [nonced transaction].
    ///
    /// [nonced transaction]: https://docs.solanalabs.com/implemented-proposals/durable-tx-nonces
    ///
    /// In this type of transaction, the blockhash is replaced with a _durable
    /// transaction nonce_, allowing for extended time to pass between the
    /// transaction's signing and submission to the blockchain.
    ///
    /// # Examples
    ///
    /// This example uses the [`solana_sdk`], [`solana_rpc_client`] and [`anyhow`] crates.
    ///
    /// [`solana_sdk`]: https://docs.rs/solana-sdk
    /// [`solana_rpc_client`]: https://docs.rs/solana-client
    /// [`anyhow`]: https://docs.rs/anyhow
    ///
    /// ```
    /// # use solana_example_mocks::{solana_keypair, solana_signer, solana_transaction};
    /// # use solana_example_mocks::solana_rpc_client;
    /// use anyhow::Result;
    /// use borsh::{BorshSerialize, BorshDeserialize};
    /// use solana_hash::Hash;
    /// use solana_instruction::Instruction;
    /// use solana_keypair::Keypair;
    /// use solana_message::Message;
    /// use solana_address::Address;
    /// use solana_rpc_client::rpc_client::RpcClient;
    /// use solana_signer::Signer;
    /// use solana_transaction::Transaction;
    /// use solana_system_interface::instruction::create_nonce_account;
    ///
    /// // A custom program instruction. This would typically be defined in
    /// // another crate so it can be shared between the on-chain program and
    /// // the client.
    /// #[derive(BorshSerialize, BorshDeserialize)]
    /// # #[borsh(crate = "borsh")]
    /// enum BankInstruction {
    ///     Initialize,
    ///     Deposit { lamports: u64 },
    ///     Withdraw { lamports: u64 },
    /// }
    ///
    /// // Create a nonced transaction for later signing and submission,
    /// // returning it and the nonce account's pubkey.
    /// fn create_offline_initialize_tx(
    ///     client: &RpcClient,
    ///     program_id: Address,
    ///     payer: &Keypair
    /// ) -> Result<(Transaction, Address)> {
    ///
    ///     let bank_instruction = BankInstruction::Initialize;
    ///     let bank_instruction = Instruction::new_with_borsh(
    ///         program_id,
    ///         &bank_instruction,
    ///         vec![],
    ///     );
    ///
    ///     // This will create a nonce account and assign authority to the
    ///     // payer so they can sign to advance the nonce and withdraw its rent.
    ///     let nonce_account = make_nonce_account(client, payer)?;
    ///
    ///     let mut message = Message::new_with_nonce(
    ///         vec![bank_instruction],
    ///         Some(&payer.pubkey()),
    ///         &nonce_account,
    ///         &payer.pubkey()
    ///     );
    ///
    ///     // This transaction will need to be signed later, using the blockhash
    ///     // stored in the nonce account.
    ///     let tx = Transaction::new_unsigned(message);
    ///
    ///     Ok((tx, nonce_account))
    /// }
    ///
    /// fn make_nonce_account(client: &RpcClient, payer: &Keypair)
    ///     -> Result<Address>
    /// {
    ///     let nonce_account_address = Keypair::new();
    ///     let nonce_account_size = solana_nonce::state::State::size();
    ///     let nonce_rent = client.get_minimum_balance_for_rent_exemption(nonce_account_size)?;
    ///
    ///     // Assigning the nonce authority to the payer so they can sign for the withdrawal,
    ///     // and we can throw away the nonce address secret key.
    ///     let create_nonce_instr = create_nonce_account(
    ///         &payer.pubkey(),
    ///         &nonce_account_address.pubkey(),
    ///         &payer.pubkey(),
    ///         nonce_rent,
    ///     );
    ///
    ///     let mut nonce_tx = Transaction::new_with_payer(&create_nonce_instr, Some(&payer.pubkey()));
    ///     let blockhash = client.get_latest_blockhash()?;
    ///     nonce_tx.sign(&[&payer, &nonce_account_address], blockhash);
    ///     client.send_and_confirm_transaction(&nonce_tx)?;
    ///
    ///     Ok(nonce_account_address.pubkey())
    /// }
    /// #
    /// # let client = RpcClient::new(String::new());
    /// # let program_id = Address::new_unique();
    /// # let payer = Keypair::new();
    /// # create_offline_initialize_tx(&client, program_id, &payer)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn new_with_nonce(
        mut instructions: Vec<Instruction>,
        payer: Option<&Address>,
        nonce_account_pubkey: &Address,
        nonce_authority_pubkey: &Address,
    ) -> Self {
        let nonce_ix =
            advance_nonce_account_instruction(nonce_account_pubkey, nonce_authority_pubkey);
        instructions.insert(0, nonce_ix);
        Self::new(&instructions, payer)
    }

    pub fn new_with_compiled_instructions(
        num_required_signatures: u8,
        num_readonly_signed_accounts: u8,
        num_readonly_unsigned_accounts: u8,
        account_keys: Vec<Address>,
        recent_blockhash: Hash,
        instructions: Vec<CompiledInstruction>,
    ) -> Self {
        Self {
            header: MessageHeader {
                num_required_signatures,
                num_readonly_signed_accounts,
                num_readonly_unsigned_accounts,
            },
            account_keys,
            recent_blockhash,
            instructions,
        }
    }

    /// Compute the blake3 hash of this transaction's message.
    #[cfg(all(not(target_os = "solana"), feature = "bincode", feature = "blake3"))]
    pub fn hash(&self) -> Hash {
        let message_bytes = self.serialize();
        Self::hash_raw_message(&message_bytes)
    }

    /// Compute the blake3 hash of a raw transaction message.
    #[cfg(all(not(target_os = "solana"), feature = "blake3"))]
    pub fn hash_raw_message(message_bytes: &[u8]) -> Hash {
        use {blake3::traits::digest::Digest, solana_hash::HASH_BYTES};
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"solana-tx-message-v1");
        hasher.update(message_bytes);
        let hash_bytes: [u8; HASH_BYTES] = hasher.finalize().into();
        hash_bytes.into()
    }

    pub fn compile_instruction(&self, ix: &Instruction) -> CompiledInstruction {
        compile_instruction(ix, &self.account_keys)
    }

    #[cfg(feature = "bincode")]
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn program_id(&self, instruction_index: usize) -> Option<&Address> {
        Some(
            &self.account_keys[self.instructions.get(instruction_index)?.program_id_index as usize],
        )
    }

    pub fn program_index(&self, instruction_index: usize) -> Option<usize> {
        Some(self.instructions.get(instruction_index)?.program_id_index as usize)
    }

    pub fn program_ids(&self) -> Vec<&Address> {
        self.instructions
            .iter()
            .map(|ix| &self.account_keys[ix.program_id_index as usize])
            .collect()
    }

    /// Returns true if the account at the specified index is an account input
    /// to some program instruction in this message.
    pub fn is_instruction_account(&self, key_index: usize) -> bool {
        if let Ok(key_index) = u8::try_from(key_index) {
            self.instructions
                .iter()
                .any(|ix| ix.accounts.contains(&key_index))
        } else {
            false
        }
    }

    pub fn is_key_called_as_program(&self, key_index: usize) -> bool {
        if let Ok(key_index) = u8::try_from(key_index) {
            self.instructions
                .iter()
                .any(|ix| ix.program_id_index == key_index)
        } else {
            false
        }
    }

    pub fn program_position(&self, index: usize) -> Option<usize> {
        let program_ids = self.program_ids();
        program_ids
            .iter()
            .position(|&&pubkey| pubkey == self.account_keys[index])
    }

    pub fn maybe_executable(&self, i: usize) -> bool {
        self.program_position(i).is_some()
    }

    pub fn demote_program_id(&self, i: usize) -> bool {
        self.is_key_called_as_program(i) && !self.is_upgradeable_loader_present()
    }

    /// Returns true if the account at the specified index was requested to be
    /// writable. This method should not be used directly.
    pub(super) fn is_writable_index(&self, i: usize) -> bool {
        i < (self.header.num_required_signatures as usize)
            .saturating_sub(self.header.num_readonly_signed_accounts as usize)
            || (i >= self.header.num_required_signatures as usize
                && i < self
                    .account_keys
                    .len()
                    .saturating_sub(self.header.num_readonly_unsigned_accounts as usize))
    }

    /// Returns true if the account at the specified index is writable by the
    /// instructions in this message. The `reserved_account_keys` param has been
    /// optional to allow clients to approximate writability without requiring
    /// fetching the latest set of reserved account keys. If this method is
    /// called by the runtime, the latest set of reserved account keys must be
    /// passed.
    pub fn is_maybe_writable(
        &self,
        i: usize,
        reserved_account_keys: Option<&HashSet<Address>>,
    ) -> bool {
        (self.is_writable_index(i))
            && !self.is_account_maybe_reserved(i, reserved_account_keys)
            && !self.demote_program_id(i)
    }

    /// Returns true if the account at the specified index is in the optional
    /// reserved account keys set.
    fn is_account_maybe_reserved(
        &self,
        key_index: usize,
        reserved_account_keys: Option<&HashSet<Address>>,
    ) -> bool {
        let mut is_maybe_reserved = false;
        if let Some(reserved_account_keys) = reserved_account_keys {
            if let Some(key) = self.account_keys.get(key_index) {
                is_maybe_reserved = reserved_account_keys.contains(key);
            }
        }
        is_maybe_reserved
    }

    pub fn is_signer(&self, i: usize) -> bool {
        i < self.header.num_required_signatures as usize
    }

    pub fn signer_keys(&self) -> Vec<&Address> {
        // Clamp in case we're working on un-`sanitize()`ed input
        let last_key = self
            .account_keys
            .len()
            .min(self.header.num_required_signatures as usize);
        self.account_keys[..last_key].iter().collect()
    }

    /// Returns `true` if `account_keys` has any duplicate keys.
    pub fn has_duplicates(&self) -> bool {
        // Note: This is an O(n^2) algorithm, but requires no heap allocations. The benchmark
        // `bench_has_duplicates` in benches/message_processor.rs shows that this implementation is
        // ~50 times faster than using HashSet for very short slices.
        for i in 1..self.account_keys.len() {
            #[allow(clippy::arithmetic_side_effects)]
            if self.account_keys[i..].contains(&self.account_keys[i - 1]) {
                return true;
            }
        }
        false
    }

    /// Returns `true` if any account is the BPF upgradeable loader.
    pub fn is_upgradeable_loader_present(&self) -> bool {
        self.account_keys
            .iter()
            .any(|&key| key == bpf_loader_upgradeable::id())
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::MESSAGE_HEADER_LENGTH,
        solana_instruction::AccountMeta,
        std::{collections::HashSet, str::FromStr},
    };

    #[test]
    // Ensure there's a way to calculate the number of required signatures.
    fn test_message_signed_keys_len() {
        let program_id = Address::default();
        let id0 = Address::default();
        let ix = Instruction::new_with_bincode(program_id, &0, vec![AccountMeta::new(id0, false)]);
        let message = Message::new(&[ix], None);
        assert_eq!(message.header.num_required_signatures, 0);

        let ix = Instruction::new_with_bincode(program_id, &0, vec![AccountMeta::new(id0, true)]);
        let message = Message::new(&[ix], Some(&id0));
        assert_eq!(message.header.num_required_signatures, 1);
    }

    #[test]
    fn test_message_kitchen_sink() {
        let program_id0 = Address::new_unique();
        let program_id1 = Address::new_unique();
        let id0 = Address::default();
        let id1 = Address::new_unique();
        let message = Message::new(
            &[
                Instruction::new_with_bincode(program_id0, &0, vec![AccountMeta::new(id0, false)]),
                Instruction::new_with_bincode(program_id1, &0, vec![AccountMeta::new(id1, true)]),
                Instruction::new_with_bincode(program_id0, &0, vec![AccountMeta::new(id1, false)]),
            ],
            Some(&id1),
        );
        assert_eq!(
            message.instructions[0],
            CompiledInstruction::new(2, &0, vec![1])
        );
        assert_eq!(
            message.instructions[1],
            CompiledInstruction::new(3, &0, vec![0])
        );
        assert_eq!(
            message.instructions[2],
            CompiledInstruction::new(2, &0, vec![0])
        );
    }

    #[test]
    fn test_message_payer_first() {
        let program_id = Address::default();
        let payer = Address::new_unique();
        let id0 = Address::default();

        let ix = Instruction::new_with_bincode(program_id, &0, vec![AccountMeta::new(id0, false)]);
        let message = Message::new(&[ix], Some(&payer));
        assert_eq!(message.header.num_required_signatures, 1);

        let ix = Instruction::new_with_bincode(program_id, &0, vec![AccountMeta::new(id0, true)]);
        let message = Message::new(&[ix], Some(&payer));
        assert_eq!(message.header.num_required_signatures, 2);

        let ix = Instruction::new_with_bincode(
            program_id,
            &0,
            vec![AccountMeta::new(payer, true), AccountMeta::new(id0, true)],
        );
        let message = Message::new(&[ix], Some(&payer));
        assert_eq!(message.header.num_required_signatures, 2);
    }

    #[test]
    fn test_program_position() {
        let program_id0 = Address::default();
        let program_id1 = Address::new_unique();
        let id = Address::new_unique();
        let message = Message::new(
            &[
                Instruction::new_with_bincode(program_id0, &0, vec![AccountMeta::new(id, false)]),
                Instruction::new_with_bincode(program_id1, &0, vec![AccountMeta::new(id, true)]),
            ],
            Some(&id),
        );
        assert_eq!(message.program_position(0), None);
        assert_eq!(message.program_position(1), Some(0));
        assert_eq!(message.program_position(2), Some(1));
    }

    #[test]
    fn test_is_maybe_writable() {
        let key0 = Address::new_unique();
        let key1 = Address::new_unique();
        let key2 = Address::new_unique();
        let key3 = Address::new_unique();
        let key4 = Address::new_unique();
        let key5 = Address::new_unique();

        let message = Message {
            header: MessageHeader {
                num_required_signatures: 3,
                num_readonly_signed_accounts: 2,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![key0, key1, key2, key3, key4, key5],
            recent_blockhash: Hash::default(),
            instructions: vec![],
        };

        let reserved_account_keys = HashSet::from([key3]);

        assert!(message.is_maybe_writable(0, Some(&reserved_account_keys)));
        assert!(!message.is_maybe_writable(1, Some(&reserved_account_keys)));
        assert!(!message.is_maybe_writable(2, Some(&reserved_account_keys)));
        assert!(!message.is_maybe_writable(3, Some(&reserved_account_keys)));
        assert!(message.is_maybe_writable(3, None));
        assert!(message.is_maybe_writable(4, Some(&reserved_account_keys)));
        assert!(!message.is_maybe_writable(5, Some(&reserved_account_keys)));
        assert!(!message.is_maybe_writable(6, Some(&reserved_account_keys)));
    }

    #[test]
    fn test_is_account_maybe_reserved() {
        let key0 = Address::new_unique();
        let key1 = Address::new_unique();

        let message = Message {
            account_keys: vec![key0, key1],
            ..Message::default()
        };

        let reserved_account_keys = HashSet::from([key1]);

        assert!(!message.is_account_maybe_reserved(0, Some(&reserved_account_keys)));
        assert!(message.is_account_maybe_reserved(1, Some(&reserved_account_keys)));
        assert!(!message.is_account_maybe_reserved(2, Some(&reserved_account_keys)));
        assert!(!message.is_account_maybe_reserved(0, None));
        assert!(!message.is_account_maybe_reserved(1, None));
        assert!(!message.is_account_maybe_reserved(2, None));
    }

    #[test]
    fn test_program_ids() {
        let key0 = Address::new_unique();
        let key1 = Address::new_unique();
        let loader2 = Address::new_unique();
        let instructions = vec![CompiledInstruction::new(2, &(), vec![0, 1])];
        let message = Message::new_with_compiled_instructions(
            1,
            0,
            2,
            vec![key0, key1, loader2],
            Hash::default(),
            instructions,
        );
        assert_eq!(message.program_ids(), vec![&loader2]);
    }

    #[test]
    fn test_is_instruction_account() {
        let key0 = Address::new_unique();
        let key1 = Address::new_unique();
        let loader2 = Address::new_unique();
        let instructions = vec![CompiledInstruction::new(2, &(), vec![0, 1])];
        let message = Message::new_with_compiled_instructions(
            1,
            0,
            2,
            vec![key0, key1, loader2],
            Hash::default(),
            instructions,
        );

        assert!(message.is_instruction_account(0));
        assert!(message.is_instruction_account(1));
        assert!(!message.is_instruction_account(2));
    }

    #[test]
    fn test_message_header_len_constant() {
        assert_eq!(
            bincode::serialized_size(&MessageHeader::default()).unwrap() as usize,
            MESSAGE_HEADER_LENGTH
        );
    }

    #[test]
    fn test_message_hash() {
        // when this test fails, it's most likely due to a new serialized format of a message.
        // in this case, the domain prefix `solana-tx-message-v1` should be updated.
        let program_id0 = Address::from_str("4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM").unwrap();
        let program_id1 = Address::from_str("8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh").unwrap();
        let id0 = Address::from_str("CiDwVBFgWV9E5MvXWoLgnEgn2hK7rJikbvfWavzAQz3").unwrap();
        let id1 = Address::from_str("GcdayuLaLyrdmUu324nahyv33G5poQdLUEZ1nEytDeP").unwrap();
        let id2 = Address::from_str("LX3EUdRUBUa3TbsYXLEUdj9J3prXkWXvLYSWyYyc2Jj").unwrap();
        let id3 = Address::from_str("QRSsyMWN1yHT9ir42bgNZUNZ4PdEhcSWCrL2AryKpy5").unwrap();
        let instructions = vec![
            Instruction::new_with_bincode(program_id0, &0, vec![AccountMeta::new(id0, false)]),
            Instruction::new_with_bincode(program_id0, &0, vec![AccountMeta::new(id1, true)]),
            Instruction::new_with_bincode(
                program_id1,
                &0,
                vec![AccountMeta::new_readonly(id2, false)],
            ),
            Instruction::new_with_bincode(
                program_id1,
                &0,
                vec![AccountMeta::new_readonly(id3, true)],
            ),
        ];

        let message = Message::new(&instructions, Some(&id1));
        assert_eq!(
            message.hash(),
            Hash::from_str("7VWCF4quo2CcWQFNUayZiorxpiR5ix8YzLebrXKf3fMF").unwrap()
        )
    }

    #[test]
    fn test_is_writable_index_saturating_behavior() {
        // Directly matching issue #150 PoC 1:
        // num_readonly_signed_accounts > num_required_signatures
        // This now results in the first part of the OR condition in is_writable_index effectively becoming `i < 0`.
        let key0 = Address::new_unique();
        let message1 = Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 2, // 2 > 1
                num_readonly_unsigned_accounts: 0,
            },
            account_keys: vec![key0],
            recent_blockhash: Hash::default(),
            instructions: vec![],
        };
        assert!(!message1.is_writable_index(0));

        // Matching issue #150 PoC 2 - num_readonly_unsigned_accounts > account_keys.len()
        let key_for_poc2 = Address::new_unique();
        let message2 = Message {
            header: MessageHeader {
                num_required_signatures: 0,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 2, // 2 > account_keys.len() (1)
            },
            account_keys: vec![key_for_poc2],
            recent_blockhash: Hash::default(),
            instructions: vec![],
        };
        assert!(!message2.is_writable_index(0));

        // Scenario 3: num_readonly_unsigned_accounts > account_keys.len() with writable signed account
        // This should result in the first condition being true for the signed account
        let message3 = Message {
            header: MessageHeader {
                num_required_signatures: 1, // Writable range starts before index 1
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 2, // 2 > account_keys.len() (1)
            },
            account_keys: vec![key0],
            recent_blockhash: Hash::default(),
            instructions: vec![],
        };
        assert!(message3.is_writable_index(0));

        // Scenario 4: Both conditions, and testing an index that would rely on the second part of OR
        let key1 = Address::new_unique();
        let message4 = Message {
            header: MessageHeader {
                num_required_signatures: 1, // Writable range starts before index 1 for signed accounts
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 3, // 3 > account_keys.len() (2)
            },
            account_keys: vec![key0, key1],
            recent_blockhash: Hash::default(),
            instructions: vec![],
        };
        assert!(message4.is_writable_index(0));
        assert!(!message4.is_writable_index(1));

        // Scenario 5: num_required_signatures is 0 due to saturating_sub
        // and num_readonly_unsigned_accounts makes the second range empty
        let message5 = Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 2, // 1.saturating_sub(2) = 0
                num_readonly_unsigned_accounts: 3, // account_keys.len().saturating_sub(3) potentially 0
            },
            account_keys: vec![key0, key1], // len is 2
            recent_blockhash: Hash::default(),
            instructions: vec![],
        };
        assert!(!message5.is_writable_index(0));
        assert!(!message5.is_writable_index(1));
    }
}
