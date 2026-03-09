#![cfg_attr(docsrs, feature(doc_auto_cfg))]
//! Instructions for the [secp256k1 native program][np].
//!
//! [np]: https://docs.solanalabs.com/runtime/programs#secp256k1-program
//!
//! _This module provides low-level cryptographic building blocks that must be
//! used carefully to ensure proper security. Read this documentation and
//! accompanying links thoroughly._
//!
//! The secp26k1 native program performs flexible verification of [secp256k1]
//! ECDSA signatures, as used by Ethereum. It can verify up to 255 signatures on
//! up to 255 messages, with those signatures, messages, and their public keys
//! arbitrarily distributed across the instruction data of any instructions in
//! the same transaction as the secp256k1 instruction.
//!
//! The secp256k1 native program ID is located in the [`secp256k1_program`] module.
//!
//! The instruction is designed for Ethereum interoperability, but may be useful
//! for other purposes. It operates on Ethereum addresses, which are [`keccak`]
//! hashes of secp256k1 public keys, and internally is implemented using the
//! secp256k1 key recovery algorithm. Ethereum address can be created for
//! secp256k1 public keys with the [`eth_address_from_pubkey`] function.
//!
//! [`keccak`]: https://docs.rs/solana-sdk/latest/solana_sdk/keccak/index.html
//!
//! This instruction does not directly allow for key recovery as in Ethereum's
//! [`ecrecover`] precompile. For that Solana provides the [`secp256k1_recover`]
//! syscall.
//!
//! [secp256k1]: https://en.bitcoin.it/wiki/Secp256k1
//! [`secp256k1_program`]: https://docs.rs/solana-program/latest/solana_program/secp256k1_program/index.html
//! [`secp256k1_recover`]: https://docs.rs/solana-secp256k1-recover
//! [`ecrecover`]: https://docs.soliditylang.org/en/v0.8.14/units-and-global-variables.html?highlight=ecrecover#mathematical-and-cryptographic-functions
//!
//! Use cases for the secp256k1 instruction include:
//!
//! - Verifying Ethereum transaction signatures.
//! - Verifying Ethereum [EIP-712] signatures.
//! - Verifying arbitrary secp256k1 signatures.
//! - Signing a single message with multiple signatures.
//!
//! [EIP-712]: https://eips.ethereum.org/EIPS/eip-712
//!
//! The [`new_secp256k1_instruction_with_signature`] function is suitable for
//! building a secp256k1 program instruction for basic use cases where a single
//! message must be signed by a known secret key. For other uses cases, including
//! many Ethereum-integration use cases, construction of the secp256k1 instruction
//! must be done manually.
//!
//! # How to use this program
//!
//! Transactions that use the secp256k1 native program will typically include
//! at least two instructions: one for the secp256k1 program to verify the
//! signatures, and one for a custom program that will check that the secp256k1
//! instruction data matches what the program expects (using
//! [`load_instruction_at_checked`] or [`get_instruction_relative`]). The
//! signatures, messages, and Ethereum addresses being verified may reside in the
//! instruction data of either of these instructions, or in the instruction data
//! of one or more additional instructions, as long as those instructions are in
//! the same transaction.
//!
//! [`load_instruction_at_checked`]: https://docs.rs/solana-program/latest/solana_program/sysvar/instructions/fn.load_instruction_at_checked.html
//! [`get_instruction_relative`]: https://docs.rs/solana-program/latest/solana_program/sysvar/instructions/fn.get_instruction_relative.html
//!
//! Correct use of this program involves multiple steps, in client code and
//! program code:
//!
//! - In the client:
//!   - Sign the [`keccak`]-hashed messages with a secp256k1 ECDSA library,
//!     like the [`k256`] crate.
//!   - Build any custom instruction data that contains signature, message, or
//!     Ethereum address data that will be used by the secp256k1 instruction.
//!   - Build the secp256k1 program instruction data, specifying the number of
//!     signatures to verify, the instruction indexes within the transaction,
//!     and offsets within those instruction's data, where the signatures,
//!     messages, and Ethereum addresses are located.
//!   - Build the custom instruction for the program that will check the results
//!     of the secp256k1 native program.
//!   - Package all instructions into a single transaction and submit them.
//! - In the program:
//!   - Load the secp256k1 instruction data with
//!     [`load_instruction_at_checked`]. or [`get_instruction_relative`].
//!   - Check that the secp256k1 program ID is equal to
//!     [`secp256k1_program::ID`], so that the signature verification cannot be
//!     faked with a malicious program.
//!   - Check that the public keys and messages are the expected values per
//!     the program's requirements.
//!
//! [`secp256k1_program::ID`]: https://docs.rs/solana-program/latest/solana_program/secp256k1_program/constant.ID.html
//!
//! The signature, message, or Ethereum addresses may reside in the secp256k1
//! instruction data itself as additional data, their bytes following the bytes
//! of the protocol required by the secp256k1 instruction to locate the
//! signature, message, and Ethereum address data. This is the technique used by
//! `new_secp256k1_instruction_with_signature` for simple signature verification.
//!
//! The `solana_secp256k1_program` crate provides few APIs for building the
//! instructions and transactions necessary for properly using the secp256k1
//! native program. Many steps must be done manually.
//!
//! The `solana_program` crate provides no APIs to assist in interpreting
//! the secp256k1 instruction data. It must be done manually.
//!
//! The secp256k1 program is implemented with the [`libsecp256k1`] crate,
//! but clients may want to use the [`k256`] crate.
//!
//! [`libsecp256k1`]: https://docs.rs/libsecp256k1/latest/libsecp256k1
//! [`k256`]: https://docs.rs/k256/latest/k256
//!
//! # Layout and interpretation of the secp256k1 instruction data
//!
//! The secp256k1 instruction data contains:
//!
//! - 1 byte indicating the number of signatures to verify, 0 - 255,
//! - A number of _signature offset_ structures that indicate where in the
//!   transaction to locate each signature, message, and Ethereum address.
//! - 0 or more bytes of arbitrary data, which may contain signatures,
//!   messages or Ethereum addresses.
//!
//! The signature offset structure is defined by [`SecpSignatureOffsets`],
//! and can be serialized to the correct format with [`bincode::serialize_into`].
//! Note that the bincode format may not be stable,
//! and callers should ensure they use the same version of `bincode` as the Solana SDK.
//! This data structure is not provided to Solana programs,
//! which are expected to interpret the signature offsets manually.
//!
//! [`bincode::serialize_into`]: https://docs.rs/bincode/1.3.3/bincode/fn.serialize_into.html
//!
//! The serialized signature offset structure has the following 11-byte layout,
//! with data types in little-endian encoding.
//!
//! | index  | bytes | type  | description |
//! |--------|-------|-------|-------------|
//! | 0      | 2     | `u16` | `signature_offset` - offset to 64-byte signature plus 1-byte recovery ID. |
//! | 2      | 1     | `u8`  | `signature_offset_instruction_index` - within the transaction, the index of the transaction whose instruction data contains the signature. |
//! | 3      | 2     | `u16` | `eth_address_offset` - offset to 20-byte Ethereum address. |
//! | 5      | 1     | `u8`  | `eth_address_instruction_index` - within the transaction, the index of the instruction whose instruction data contains the Ethereum address. |
//! | 6      | 2     | `u16` | `message_data_offset` - Offset to start of message data. |
//! | 8      | 2     | `u16` | `message_data_size` - Size of message data in bytes. |
//! | 10     | 1     | `u8`  | `message_instruction_index` - Within the transaction, the index of the instruction whose instruction data contains the message data. |
//!
//! # Signature malleability
//!
//! With the ECDSA signature algorithm it is possible for any party, given a
//! valid signature of some message, to create a second signature that is
//! equally valid. This is known as _signature malleability_. In many cases this
//! is not a concern, but in cases where applications rely on signatures to have
//! a unique representation this can be the source of bugs, potentially with
//! security implications.
//!
//! **The solana `secp256k1_recover` function does not prevent signature
//! malleability**. This is in contrast to the Bitcoin secp256k1 library, which
//! does prevent malleability by default. Solana accepts signatures with `S`
//! values that are either in the _high order_ or in the _low order_, and it
//! is trivial to produce one from the other.
//!
//! For more complete documentation of the subject, and techniques to prevent
//! malleability, see the documentation for the [`secp256k1_recover`] syscall.
//!
//! # Additional security considerations
//!
//! Most programs will want to be conservative about the layout of the secp256k1 instruction
//! to prevent unforeseen bugs. The following checks may be desirable:
//!
//! - That there are exactly the expected number of signatures.
//! - That the three indexes, `signature_offset_instruction_index`,
//!   `eth_address_instruction_index`, and `message_instruction_index` are as
//!   expected, placing the signature, message and Ethereum address in the
//!   expected instruction.
//!
//! Loading the secp256k1 instruction data within a program requires access to
//! the [instructions sysvar][is], which must be passed to the program by its
//! caller. Programs must verify the ID of this program to avoid calling an
//! imposter program. This does not need to be done manually though, as long as
//! it is only used through the [`load_instruction_at_checked`] or
//! [`get_instruction_relative`] functions. Both of these functions check their
//! sysvar argument to ensure it is the known instruction sysvar.
//!
//! [is]: https://docs.rs/solana-program/latest/solana_program/sysvar/instructions/index.html
//!
//! Programs should _always_ verify that the secp256k1 program ID loaded through
//! the instructions sysvar has the same value as in the [`secp256k1_program`]
//! module. Again this prevents imposter programs.
//!
//! [`secp256k1_program`]: https://docs.rs/solana-program/latest/solana_program/secp256k1_program/index.html
//!
//! # Errors
//!
//! The transaction will fail if any of the following are true:
//!
//! - Any signature was not created by the secret key corresponding to the
//!   specified public key.
//! - Any signature is invalid.
//! - Any signature is "overflowing", a non-standard condition.
//! - The instruction data is empty.
//! - The first byte of instruction data is equal to 0 (indicating no signatures),
//!   but the instruction data's length is greater than 1.
//! - The instruction data is not long enough to hold the number of signature
//!   offsets specified in the first byte.
//! - Any instruction indexes specified in the signature offsets are greater or
//!   equal to the number of instructions in the transaction.
//! - Any bounds specified in the signature offsets exceed the bounds of the
//!   instruction data to which they are indexed.
//!
//! # Examples
//!
//! Both of the following examples make use of the following module definition
//! to parse the secp256k1 instruction data from within a Solana program.
//!
//! ```no_run
//! mod secp256k1_defs {
//!     use solana_program_error::ProgramError;
//!     use std::iter::Iterator;
//!
//!     pub const HASHED_PUBKEY_SERIALIZED_SIZE: usize = 20;
//!     pub const SIGNATURE_SERIALIZED_SIZE: usize = 64;
//!     pub const SIGNATURE_OFFSETS_SERIALIZED_SIZE: usize = 11;
//!
//!     /// The structure encoded in the secp2256k1 instruction data.
//!     pub struct SecpSignatureOffsets {
//!         pub signature_offset: u16,
//!         pub signature_instruction_index: u8,
//!         pub eth_address_offset: u16,
//!         pub eth_address_instruction_index: u8,
//!         pub message_data_offset: u16,
//!         pub message_data_size: u16,
//!         pub message_instruction_index: u8,
//!     }
//!
//!     pub fn iter_signature_offsets(
//!        secp256k1_instr_data: &[u8],
//!     ) -> Result<impl Iterator<Item = SecpSignatureOffsets> + '_, ProgramError> {
//!         // First element is the number of `SecpSignatureOffsets`.
//!         let num_structs = *secp256k1_instr_data
//!             .get(0)
//!             .ok_or(ProgramError::InvalidArgument)?;
//!
//!         let all_structs_size = SIGNATURE_OFFSETS_SERIALIZED_SIZE * num_structs as usize;
//!         let all_structs_slice = secp256k1_instr_data
//!             .get(1..all_structs_size + 1)
//!             .ok_or(ProgramError::InvalidArgument)?;
//!
//!         fn decode_u16(chunk: &[u8], index: usize) -> u16 {
//!             u16::from_le_bytes(<[u8; 2]>::try_from(&chunk[index..index + 2]).unwrap())
//!         }
//!
//!         Ok(all_structs_slice
//!             .chunks(SIGNATURE_OFFSETS_SERIALIZED_SIZE)
//!             .map(|chunk| SecpSignatureOffsets {
//!                 signature_offset: decode_u16(chunk, 0),
//!                 signature_instruction_index: chunk[2],
//!                 eth_address_offset: decode_u16(chunk, 3),
//!                 eth_address_instruction_index: chunk[5],
//!                 message_data_offset: decode_u16(chunk, 6),
//!                 message_data_size: decode_u16(chunk, 8),
//!                 message_instruction_index: chunk[10],
//!             }))
//!     }
//! }
//! ```
//!
//! ## Example: Signing and verifying with `new_secp256k1_instruction_with_signature`
//!
//! This example demonstrates the simplest way to use the secp256k1 program, by
//! calling [`new_secp256k1_instruction_with_signature`] to sign a single message
//! and build the corresponding secp256k1 instruction.
//!
//! This example has two components: a Solana program, and an RPC client that
//! sends a transaction to call it. The RPC client will sign a single message,
//! and the Solana program will introspect the secp256k1 instruction to verify
//! that the signer matches a known authorized public key.
//!
//! The Solana program. Note that it uses `k256` version 0.13.0 to parse the
//! secp256k1 signature to prevent malleability.
//!
//! ```no_run
//! # mod secp256k1_defs {
//! #     use solana_program_error::ProgramError;
//! #     use std::iter::Iterator;
//! #
//! #     pub const HASHED_PUBKEY_SERIALIZED_SIZE: usize = 20;
//! #     pub const SIGNATURE_SERIALIZED_SIZE: usize = 64;
//! #     pub const SIGNATURE_OFFSETS_SERIALIZED_SIZE: usize = 11;
//! #
//! #     /// The structure encoded in the secp2256k1 instruction data.
//! #     pub struct SecpSignatureOffsets {
//! #         pub signature_offset: u16,
//! #         pub signature_instruction_index: u8,
//! #         pub eth_address_offset: u16,
//! #         pub eth_address_instruction_index: u8,
//! #         pub message_data_offset: u16,
//! #         pub message_data_size: u16,
//! #         pub message_instruction_index: u8,
//! #     }
//! #
//! #     pub fn iter_signature_offsets(
//! #        secp256k1_instr_data: &[u8],
//! #     ) -> Result<impl Iterator<Item = SecpSignatureOffsets> + '_, ProgramError> {
//! #         // First element is the number of `SecpSignatureOffsets`.
//! #         let num_structs = *secp256k1_instr_data
//! #             .get(0)
//! #             .ok_or(ProgramError::InvalidArgument)?;
//! #
//! #         let all_structs_size = SIGNATURE_OFFSETS_SERIALIZED_SIZE * num_structs as usize;
//! #         let all_structs_slice = secp256k1_instr_data
//! #             .get(1..all_structs_size + 1)
//! #             .ok_or(ProgramError::InvalidArgument)?;
//! #
//! #         fn decode_u16(chunk: &[u8], index: usize) -> u16 {
//! #             u16::from_le_bytes(<[u8; 2]>::try_from(&chunk[index..index + 2]).unwrap())
//! #         }
//! #
//! #         Ok(all_structs_slice
//! #             .chunks(SIGNATURE_OFFSETS_SERIALIZED_SIZE)
//! #             .map(|chunk| SecpSignatureOffsets {
//! #                 signature_offset: decode_u16(chunk, 0),
//! #                 signature_instruction_index: chunk[2],
//! #                 eth_address_offset: decode_u16(chunk, 3),
//! #                 eth_address_instruction_index: chunk[5],
//! #                 message_data_offset: decode_u16(chunk, 6),
//! #                 message_data_size: decode_u16(chunk, 8),
//! #                 message_instruction_index: chunk[10],
//! #             }))
//! #     }
//! # }
//! use k256::elliptic_curve::scalar::IsHigh;
//! use solana_account_info::{next_account_info, AccountInfo};
//! use solana_msg::msg;
//! use solana_program_error::{ProgramError, ProgramResult};
//! use solana_sdk_ids::secp256k1_program;
//! use solana_instructions_sysvar::load_instruction_at_checked;
//!
//! /// An Ethereum address corresponding to a secp256k1 secret key that is
//! /// authorized to sign our messages.
//! const AUTHORIZED_ETH_ADDRESS: [u8; 20] = [
//!     0x18, 0x8a, 0x5c, 0xf2, 0x3b, 0x0e, 0xff, 0xe9, 0xa8, 0xe1, 0x42, 0x64, 0x5b, 0x82, 0x2f, 0x3a,
//!     0x6b, 0x8b, 0x52, 0x35,
//! ];
//!
//! /// Check the secp256k1 instruction to ensure it was signed by
//! /// `AUTHORIZED_ETH_ADDRESS`s key.
//! ///
//! /// `accounts` is the slice of all accounts passed to the program
//! /// entrypoint. The only account it should contain is the instructions sysvar.
//! fn demo_secp256k1_verify_basic(
//!    accounts: &[AccountInfo],
//! ) -> ProgramResult {
//!     let account_info_iter = &mut accounts.iter();
//!
//!     // The instructions sysvar gives access to the instructions in the transaction.
//!     let instructions_sysvar_account = next_account_info(account_info_iter)?;
//!     assert!(solana_sdk_ids::sysvar::instructions::check_id(
//!         instructions_sysvar_account.key
//!     ));
//!
//!     // Load the secp256k1 instruction.
//!     // `new_secp256k1_instruction_with_signature` generates an instruction
//!     // that must be at index 0.
//!     let secp256k1_instr =
//!         solana_instructions_sysvar::load_instruction_at_checked(0, instructions_sysvar_account)?;
//!
//!     // Verify it is a secp256k1 instruction.
//!     // This is security-critical - what if the transaction uses an imposter secp256k1 program?
//!     assert!(secp256k1_program::check_id(&secp256k1_instr.program_id));
//!
//!     // There must be at least one byte. This is also verified by the runtime,
//!     // and doesn't strictly need to be checked.
//!     assert!(secp256k1_instr.data.len() > 1);
//!
//!     let num_signatures = secp256k1_instr.data[0];
//!     // `new_secp256k1_instruction_with_signature` generates an instruction
//!     // that contains one signature.
//!     assert_eq!(1, num_signatures);
//!
//!     // Load the first and only set of signature offsets.
//!     let offsets: secp256k1_defs::SecpSignatureOffsets =
//!         secp256k1_defs::iter_signature_offsets(&secp256k1_instr.data)?
//!             .next()
//!             .ok_or(ProgramError::InvalidArgument)?;
//!
//!     // `new_secp256k1_instruction_with_signature` generates an instruction
//!     // that only uses instruction index 0.
//!     assert_eq!(0, offsets.signature_instruction_index);
//!     assert_eq!(0, offsets.eth_address_instruction_index);
//!     assert_eq!(0, offsets.message_instruction_index);
//!
//!     // Reject high-s value signatures to prevent malleability.
//!     // Solana does not do this itself.
//!     // This may or may not be necessary depending on use case.
//!     {
//!         let signature = &secp256k1_instr.data[offsets.signature_offset as usize
//!             ..offsets.signature_offset as usize + secp256k1_defs::SIGNATURE_SERIALIZED_SIZE];
//!         let signature = k256::ecdsa::Signature::from_slice(signature)
//!             .map_err(|_| ProgramError::InvalidArgument)?;
//!
//!         if bool::from(signature.s().is_high()) {
//!             msg!("signature with high-s value");
//!             return Err(ProgramError::InvalidArgument);
//!         }
//!     }
//!
//!     // There is likely at least one more verification step a real program needs
//!     // to do here to ensure it trusts the secp256k1 instruction, e.g.:
//!     //
//!     // - verify the tx signer is authorized
//!     // - verify the secp256k1 signer is authorized
//!
//!     // Here we are checking the secp256k1 pubkey against a known authorized pubkey.
//!     let eth_address = &secp256k1_instr.data[offsets.eth_address_offset as usize
//!         ..offsets.eth_address_offset as usize + secp256k1_defs::HASHED_PUBKEY_SERIALIZED_SIZE];
//!
//!     if eth_address != AUTHORIZED_ETH_ADDRESS {
//!         return Err(ProgramError::InvalidArgument);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! The client program:
//!
//! ```no_run
//! # use solana_example_mocks::{solana_keypair, solana_rpc_client, solana_signer, solana_transaction};
//! use anyhow::Result;
//! use solana_instruction::{AccountMeta, Instruction};
//! use solana_keypair::Keypair;
//! use solana_rpc_client::rpc_client::RpcClient;
//! use solana_signer::Signer;
//! use solana_transaction::Transaction;
//! use solana_secp256k1_program::{
//!     eth_address_from_pubkey, new_secp256k1_instruction_with_signature,
//!     sign_message,
//! };
//!
//! fn demo_secp256k1_verify_basic(
//!     payer_keypair: &Keypair,
//!     secp256k1_secret_key: &k256::ecdsa::SigningKey,
//!     client: &RpcClient,
//!     program_keypair: &Keypair,
//! ) -> Result<()> {
//!     // Internally to `sign_message` and `secp256k_instruction::verify`
//!     // (the secp256k1 program), this message is keccak-hashed before signing.
//!     let msg = b"hello world";
//!     let secp_pubkey = secp256k1_secret_key.verifying_key();
//!     let eth_address = eth_address_from_pubkey(&secp_pubkey.to_encoded_point(false).as_bytes()[1..].try_into().unwrap());
//!     let (signature, recovery_id) = sign_message(&secp256k1_secret_key.to_bytes().into(), msg).unwrap();
//!    let secp256k1_instr = new_secp256k1_instruction_with_signature(msg, &signature, recovery_id, &eth_address);
//!
//!     let program_instr = Instruction::new_with_bytes(
//!         program_keypair.pubkey(),
//!         &[],
//!         vec![
//!             AccountMeta::new_readonly(solana_sdk_ids::sysvar::instructions::ID, false)
//!         ],
//!     );
//!
//!     let blockhash = client.get_latest_blockhash()?;
//!     let tx = Transaction::new_signed_with_payer(
//!         &[secp256k1_instr, program_instr],
//!         Some(&payer_keypair.pubkey()),
//!         &[payer_keypair],
//!         blockhash,
//!     );
//!
//!     client.send_and_confirm_transaction(&tx)?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Example: Verifying multiple signatures in one instruction
//!
//! This example demonstrates manually creating a secp256k1 instruction
//! containing many signatures, and a Solana program that parses them all. This
//! example on its own has no practical purpose. It simply demonstrates advanced
//! use of the secp256k1 program.
//!
//! Recall that the secp256k1 program will accept signatures, messages, and
//! Ethereum addresses that reside in any instruction contained in the same
//! transaction. In the _previous_ example, the Solana program asserted that all
//! signatures, messages, and addresses were stored in the instruction at 0. In
//! this next example the Solana program supports signatures, messages, and
//! addresses stored in any instruction. For simplicity the client still only
//! stores signatures, messages, and addresses in a single instruction, the
//! secp256k1 instruction. The code for storing this data across multiple
//! instructions would be complex, and may not be necessary in practice.
//!
//! This example has two components: a Solana program, and an RPC client that
//! sends a transaction to call it.
//!
//! The Solana program:
//!
//! ```no_run
//! # mod secp256k1_defs {
//! #     use solana_program_error::ProgramError;
//! #     use std::iter::Iterator;
//! #
//! #     pub const HASHED_PUBKEY_SERIALIZED_SIZE: usize = 20;
//! #     pub const SIGNATURE_SERIALIZED_SIZE: usize = 64;
//! #     pub const SIGNATURE_OFFSETS_SERIALIZED_SIZE: usize = 11;
//! #
//! #     /// The structure encoded in the secp2256k1 instruction data.
//! #     pub struct SecpSignatureOffsets {
//! #         pub signature_offset: u16,
//! #         pub signature_instruction_index: u8,
//! #         pub eth_address_offset: u16,
//! #         pub eth_address_instruction_index: u8,
//! #         pub message_data_offset: u16,
//! #         pub message_data_size: u16,
//! #         pub message_instruction_index: u8,
//! #     }
//! #
//! #     pub fn iter_signature_offsets(
//! #        secp256k1_instr_data: &[u8],
//! #     ) -> Result<impl Iterator<Item = SecpSignatureOffsets> + '_, ProgramError> {
//! #         // First element is the number of `SecpSignatureOffsets`.
//! #         let num_structs = *secp256k1_instr_data
//! #             .get(0)
//! #             .ok_or(ProgramError::InvalidArgument)?;
//! #
//! #         let all_structs_size = SIGNATURE_OFFSETS_SERIALIZED_SIZE * num_structs as usize;
//! #         let all_structs_slice = secp256k1_instr_data
//! #             .get(1..all_structs_size + 1)
//! #             .ok_or(ProgramError::InvalidArgument)?;
//! #
//! #         fn decode_u16(chunk: &[u8], index: usize) -> u16 {
//! #             u16::from_le_bytes(<[u8; 2]>::try_from(&chunk[index..index + 2]).unwrap())
//! #         }
//! #
//! #         Ok(all_structs_slice
//! #             .chunks(SIGNATURE_OFFSETS_SERIALIZED_SIZE)
//! #             .map(|chunk| SecpSignatureOffsets {
//! #                 signature_offset: decode_u16(chunk, 0),
//! #                 signature_instruction_index: chunk[2],
//! #                 eth_address_offset: decode_u16(chunk, 3),
//! #                 eth_address_instruction_index: chunk[5],
//! #                 message_data_offset: decode_u16(chunk, 6),
//! #                 message_data_size: decode_u16(chunk, 8),
//! #                 message_instruction_index: chunk[10],
//! #             }))
//! #     }
//! # }
//! use solana_account_info::{next_account_info, AccountInfo};
//! use solana_program_error::{ProgramError, ProgramResult};
//! use solana_msg::msg;
//! use solana_sdk_ids::secp256k1_program;
//! use solana_instructions_sysvar::{get_instruction_relative, load_instruction_at_checked};
//!
//! /// A struct to hold the values specified in the `SecpSignatureOffsets` struct.
//! struct SecpSignature {
//!     signature: [u8; secp256k1_defs::SIGNATURE_SERIALIZED_SIZE],
//!     recovery_id: u8,
//!     eth_address: [u8; secp256k1_defs::HASHED_PUBKEY_SERIALIZED_SIZE],
//!     message: Vec<u8>,
//! }
//!
//! /// Load all signatures indicated in the secp256k1 instruction.
//! ///
//! /// This function is quite inefficient for reloading the same instructions
//! /// repeatedly and making copies and allocations.
//! fn load_signatures(
//!     secp256k1_instr_data: &[u8],
//!     instructions_sysvar_account: &AccountInfo,
//! ) -> Result<Vec<SecpSignature>, ProgramError> {
//!     let mut sigs = vec![];
//!     for offsets in secp256k1_defs::iter_signature_offsets(secp256k1_instr_data)? {
//!         let signature_instr = load_instruction_at_checked(
//!             offsets.signature_instruction_index as usize,
//!             instructions_sysvar_account,
//!         )?;
//!         let eth_address_instr = load_instruction_at_checked(
//!             offsets.eth_address_instruction_index as usize,
//!             instructions_sysvar_account,
//!         )?;
//!         let message_instr = load_instruction_at_checked(
//!             offsets.message_instruction_index as usize,
//!             instructions_sysvar_account,
//!         )?;
//!
//!         // These indexes must all be valid because the runtime already verified them.
//!         let signature = &signature_instr.data[offsets.signature_offset as usize
//!             ..offsets.signature_offset as usize + secp256k1_defs::SIGNATURE_SERIALIZED_SIZE];
//!         let recovery_id = signature_instr.data
//!             [offsets.signature_offset as usize + secp256k1_defs::SIGNATURE_SERIALIZED_SIZE];
//!         let eth_address = &eth_address_instr.data[offsets.eth_address_offset as usize
//!             ..offsets.eth_address_offset as usize + secp256k1_defs::HASHED_PUBKEY_SERIALIZED_SIZE];
//!         let message = &message_instr.data[offsets.message_data_offset as usize
//!             ..offsets.message_data_offset as usize + offsets.message_data_size as usize];
//!
//!         let signature =
//!             <[u8; secp256k1_defs::SIGNATURE_SERIALIZED_SIZE]>::try_from(signature).unwrap();
//!         let eth_address =
//!             <[u8; secp256k1_defs::HASHED_PUBKEY_SERIALIZED_SIZE]>::try_from(eth_address).unwrap();
//!         let message = Vec::from(message);
//!
//!         sigs.push(SecpSignature {
//!             signature,
//!             recovery_id,
//!             eth_address,
//!             message,
//!         })
//!     }
//!     Ok(sigs)
//! }
//!
//! fn demo_secp256k1_custom_many(
//!     accounts: &[AccountInfo],
//! ) -> ProgramResult {
//!     let account_info_iter = &mut accounts.iter();
//!
//!     let instructions_sysvar_account = next_account_info(account_info_iter)?;
//!     assert!(solana_sdk_ids::sysvar::instructions::check_id(
//!         instructions_sysvar_account.key
//!     ));
//!
//!     let secp256k1_instr =
//!         solana_instructions_sysvar::get_instruction_relative(-1, instructions_sysvar_account)?;
//!
//!     assert!(secp256k1_program::check_id(&secp256k1_instr.program_id));
//!
//!     let signatures = load_signatures(&secp256k1_instr.data, instructions_sysvar_account)?;
//!     for (idx, signature_bundle) in signatures.iter().enumerate() {
//!         let signature = hex::encode(&signature_bundle.signature);
//!         let eth_address = hex::encode(&signature_bundle.eth_address);
//!         let message = hex::encode(&signature_bundle.message);
//!         msg!("sig {}: {:?}", idx, signature);
//!         msg!("recid: {}: {}", idx, signature_bundle.recovery_id);
//!         msg!("eth address {}: {}", idx, eth_address);
//!         msg!("message {}: {}", idx, message);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! The client program:
//!
//! ```no_run
//! # use solana_example_mocks::{solana_keypair, solana_rpc_client, solana_signer, solana_transaction};
//! use anyhow::Result;
//! use solana_instruction::{AccountMeta, Instruction};
//! use solana_rpc_client::rpc_client::RpcClient;
//! use solana_secp256k1_program::{
//!     eth_address_from_pubkey, SecpSignatureOffsets, HASHED_PUBKEY_SERIALIZED_SIZE,
//!     SIGNATURE_OFFSETS_SERIALIZED_SIZE, SIGNATURE_SERIALIZED_SIZE,
//! };
//! use solana_signer::Signer;
//! use solana_keypair::Keypair;
//! use solana_transaction::Transaction;
//!
//! /// A struct to hold the values specified in the `SecpSignatureOffsets` struct.
//! struct SecpSignature {
//!     signature: [u8; SIGNATURE_SERIALIZED_SIZE],
//!     recovery_id: u8,
//!     eth_address: [u8; HASHED_PUBKEY_SERIALIZED_SIZE],
//!     message: Vec<u8>,
//! }
//!
//! /// Create the instruction data for a secp256k1 instruction.
//! ///
//! /// `instruction_index` is the index the secp256k1 instruction will appear
//! /// within the transaction. For simplicity, this function only supports packing
//! /// the signatures into the secp256k1 instruction data, and not into any other
//! /// instructions within the transaction.
//! fn make_secp256k1_instruction_data(
//!     signatures: &[SecpSignature],
//!     instruction_index: u8,
//! ) -> Result<Vec<u8>> {
//!     assert!(signatures.len() <= u8::MAX.into());
//!
//!     // We're going to pack all the signatures into the secp256k1 instruction data.
//!     // Before our signatures though is the signature offset structures
//!     // the secp256k1 program parses to find those signatures.
//!     // This value represents the byte offset where the signatures begin.
//!     let data_start = 1 + signatures.len() * SIGNATURE_OFFSETS_SERIALIZED_SIZE;
//!
//!     let mut signature_offsets = vec![];
//!     let mut signature_buffer = vec![];
//!
//!     for signature_bundle in signatures {
//!         let data_start = data_start
//!             .checked_add(signature_buffer.len())
//!             .expect("overflow");
//!
//!         let signature_offset = data_start;
//!         let eth_address_offset = data_start
//!             .checked_add(SIGNATURE_SERIALIZED_SIZE + 1)
//!             .expect("overflow");
//!         let message_data_offset = eth_address_offset
//!             .checked_add(HASHED_PUBKEY_SERIALIZED_SIZE)
//!             .expect("overflow");
//!         let message_data_size = signature_bundle.message.len();
//!
//!         let signature_offset = u16::try_from(signature_offset)?;
//!         let eth_address_offset = u16::try_from(eth_address_offset)?;
//!         let message_data_offset = u16::try_from(message_data_offset)?;
//!         let message_data_size = u16::try_from(message_data_size)?;
//!
//!         signature_offsets.push(SecpSignatureOffsets {
//!             signature_offset,
//!             signature_instruction_index: instruction_index,
//!             eth_address_offset,
//!             eth_address_instruction_index: instruction_index,
//!             message_data_offset,
//!             message_data_size,
//!             message_instruction_index: instruction_index,
//!         });
//!
//!         signature_buffer.extend(signature_bundle.signature);
//!         signature_buffer.push(signature_bundle.recovery_id);
//!         signature_buffer.extend(&signature_bundle.eth_address);
//!         signature_buffer.extend(&signature_bundle.message);
//!     }
//!
//!     let mut instr_data = vec![];
//!     instr_data.push(signatures.len() as u8);
//!
//!     for offsets in signature_offsets {
//!         let offsets = bincode::serialize(&offsets)?;
//!         instr_data.extend(offsets);
//!     }
//!
//!     instr_data.extend(signature_buffer);
//!
//!     Ok(instr_data)
//! }
//!
//! fn demo_secp256k1_custom_many(
//!     payer_keypair: &Keypair,
//!     client: &RpcClient,
//!     program_keypair: &Keypair,
//! ) -> Result<()> {
//!     // Sign some messages.
//!     let mut signatures = vec![];
//!     for idx in 0..2 {
//!         let secret_key = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
//!         let message = format!("hello world {}", idx).into_bytes();
//!         let message_hash = {
//!             let mut hasher = solana_keccak_hasher::Hasher::default();
//!             hasher.hash(&message);
//!             hasher.result()
//!         };
//!         let (signature, recovery_id) =
//!             secret_key.sign_prehash_recoverable(message_hash.as_bytes()).unwrap();
//!         let signature = signature.to_bytes().into();
//!         let recovery_id = recovery_id.to_byte();
//!
//!         let public_key = secret_key.verifying_key();
//!         let eth_address =
//!             eth_address_from_pubkey(&public_key.to_encoded_point(false).as_bytes()[1..].try_into().unwrap());
//!
//!         signatures.push(SecpSignature {
//!             signature,
//!             recovery_id,
//!             eth_address,
//!             message,
//!         });
//!     }
//!
//!     let secp256k1_instr_data = make_secp256k1_instruction_data(&signatures, 0)?;
//!     let secp256k1_instr = Instruction::new_with_bytes(
//!         solana_sdk_ids::secp256k1_program::ID,
//!         &secp256k1_instr_data,
//!         vec![],
//!     );
//!
//!     let program_instr = Instruction::new_with_bytes(
//!         program_keypair.pubkey(),
//!         &[],
//!         vec![
//!             AccountMeta::new_readonly(solana_sdk_ids::sysvar::instructions::ID, false)
//!         ],
//!     );
//!
//!     let blockhash = client.get_latest_blockhash()?;
//!     let tx = Transaction::new_signed_with_payer(
//!         &[secp256k1_instr, program_instr],
//!         Some(&payer_keypair.pubkey()),
//!         &[payer_keypair],
//!         blockhash,
//!     );
//!
//!     client.send_and_confirm_transaction(&tx)?;
//!
//!     Ok(())
//! }
//! ```

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};
#[cfg(feature = "bincode")]
use solana_instruction::Instruction;
use {digest::Digest, solana_signature::error::Error};

pub const SECP256K1_PUBKEY_SIZE: usize = 64;
pub const SECP256K1_PRIVATE_KEY_SIZE: usize = 32;
pub const HASHED_PUBKEY_SERIALIZED_SIZE: usize = 20;

pub const SIGNATURE_SERIALIZED_SIZE: usize = 64;
pub const SIGNATURE_OFFSETS_SERIALIZED_SIZE: usize = 11;
pub const DATA_START: usize = SIGNATURE_OFFSETS_SERIALIZED_SIZE + 1;

/// Offsets of signature data within a secp256k1 instruction.
///
/// See the [module documentation][md] for a complete description.
///
/// [md]: self
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Default, Debug, Eq, PartialEq)]
pub struct SecpSignatureOffsets {
    /// Offset to 64-byte signature plus 1-byte recovery ID.
    pub signature_offset: u16,
    /// Within the transaction, the index of the instruction whose instruction data contains the signature.
    pub signature_instruction_index: u8,
    /// Offset to 20-byte Ethereum address.
    pub eth_address_offset: u16,
    /// Within the transaction, the index of the instruction whose instruction data contains the address.
    pub eth_address_instruction_index: u8,
    /// Offset to start of message data.
    pub message_data_offset: u16,
    /// Size of message data in bytes.
    pub message_data_size: u16,
    /// Within the transaction, the index of the instruction whose instruction data contains the message.
    pub message_instruction_index: u8,
}

/// Signs a message from the given private key bytes
pub fn sign_message(
    priv_key_bytes: &[u8; SECP256K1_PRIVATE_KEY_SIZE],
    message: &[u8],
) -> Result<([u8; SIGNATURE_SERIALIZED_SIZE], u8), Error> {
    let priv_key = k256::ecdsa::SigningKey::from_slice(priv_key_bytes)
        .map_err(|e| Error::from_source(format!("{e}")))?;
    let mut hasher = sha3::Keccak256::new();
    hasher.update(message);
    let message_hash = hasher.finalize();
    let mut message_hash_arr = [0u8; 32];
    message_hash_arr.copy_from_slice(message_hash.as_slice());
    let (signature, recovery_id) = priv_key
        .sign_prehash_recoverable(&message_hash_arr)
        .map_err(|e| Error::from_source(format!("{e}")))?;
    Ok((signature.to_bytes().into(), recovery_id.to_byte()))
}

#[cfg(feature = "bincode")]
pub fn new_secp256k1_instruction_with_signature(
    message_arr: &[u8],
    signature: &[u8; SIGNATURE_SERIALIZED_SIZE],
    recovery_id: u8,
    eth_address: &[u8; HASHED_PUBKEY_SERIALIZED_SIZE],
) -> Instruction {
    let instruction_data_len = DATA_START
        .saturating_add(eth_address.len())
        .saturating_add(signature.len())
        .saturating_add(message_arr.len())
        .saturating_add(1);
    let mut instruction_data = vec![0; instruction_data_len];

    let eth_address_offset = DATA_START;
    instruction_data[eth_address_offset..eth_address_offset.saturating_add(eth_address.len())]
        .copy_from_slice(eth_address);

    let signature_offset = DATA_START.saturating_add(eth_address.len());
    instruction_data[signature_offset..signature_offset.saturating_add(signature.len())]
        .copy_from_slice(signature);

    instruction_data[signature_offset.saturating_add(signature.len())] = recovery_id;

    let message_data_offset = signature_offset
        .saturating_add(signature.len())
        .saturating_add(1);
    instruction_data[message_data_offset..].copy_from_slice(message_arr);

    let num_signatures = 1;
    instruction_data[0] = num_signatures;
    let offsets = SecpSignatureOffsets {
        signature_offset: signature_offset as u16,
        signature_instruction_index: 0,
        eth_address_offset: eth_address_offset as u16,
        eth_address_instruction_index: 0,
        message_data_offset: message_data_offset as u16,
        message_data_size: message_arr.len() as u16,
        message_instruction_index: 0,
    };
    let writer = std::io::Cursor::new(&mut instruction_data[1..DATA_START]);
    bincode::serialize_into(writer, &offsets).unwrap();

    Instruction {
        program_id: solana_sdk_ids::secp256k1_program::id(),
        accounts: vec![],
        data: instruction_data,
    }
}

/// Creates an Ethereum address from a secp256k1 public key.
pub fn eth_address_from_pubkey(
    pubkey: &[u8; SECP256K1_PUBKEY_SIZE],
) -> [u8; HASHED_PUBKEY_SERIALIZED_SIZE] {
    let mut addr = [0u8; HASHED_PUBKEY_SERIALIZED_SIZE];
    addr.copy_from_slice(&sha3::Keccak256::digest(pubkey)[12..]);
    assert_eq!(addr.len(), HASHED_PUBKEY_SERIALIZED_SIZE);
    addr
}
