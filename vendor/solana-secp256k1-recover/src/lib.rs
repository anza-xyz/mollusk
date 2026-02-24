#![cfg_attr(feature = "frozen-abi", feature(min_specialization))]
#![cfg_attr(docsrs, feature(doc_cfg))]
//! Public key recovery from [secp256k1] ECDSA signatures.
//!
//! [secp256k1]: https://en.bitcoin.it/wiki/Secp256k1
//!
//! _This module provides low-level cryptographic building blocks that must be
//! used carefully to ensure proper security. Read this documentation and
//! accompanying links thoroughly._
//!
//! The [`secp256k1_recover`] syscall allows a secp256k1 public key that has
//! previously signed a message to be recovered from the combination of the
//! message, the signature, and a recovery ID. The recovery ID is generated
//! during signing.
//!
//! Use cases for `secp256k1_recover` include:
//!
//! - Implementing the Ethereum [`ecrecover`] builtin contract.
//! - Performing secp256k1 public key recovery generally.
//! - Verifying a single secp256k1 signature.
//!
//! While `secp256k1_recover` can be used to verify secp256k1 signatures, Solana
//! also provides the [secp256k1 program][sp], which is more flexible, has lower CPU
//! cost, and can validate many signatures at once.
//!
//! [sp]: https://docs.rs/solana-program/latest/solana_program/secp256k1_program/
//! [`ecrecover`]: https://docs.soliditylang.org/en/v0.8.14/units-and-global-variables.html?highlight=ecrecover#mathematical-and-cryptographic-functions

#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use {core::convert::TryFrom, thiserror::Error};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Secp256k1RecoverError {
    #[error("The hash provided to a secp256k1_recover is invalid")]
    InvalidHash,
    #[error("The recovery_id provided to a secp256k1_recover is invalid")]
    InvalidRecoveryId,
    #[error("The signature provided to a secp256k1_recover is invalid")]
    InvalidSignature,
}

impl From<u64> for Secp256k1RecoverError {
    fn from(v: u64) -> Secp256k1RecoverError {
        match v {
            1 => Secp256k1RecoverError::InvalidHash,
            2 => Secp256k1RecoverError::InvalidRecoveryId,
            3 => Secp256k1RecoverError::InvalidSignature,
            _ => panic!("Unsupported Secp256k1RecoverError"),
        }
    }
}

impl From<Secp256k1RecoverError> for u64 {
    fn from(v: Secp256k1RecoverError) -> u64 {
        match v {
            Secp256k1RecoverError::InvalidHash => 1,
            Secp256k1RecoverError::InvalidRecoveryId => 2,
            Secp256k1RecoverError::InvalidSignature => 3,
        }
    }
}

pub const SECP256K1_SIGNATURE_LENGTH: usize = 64;
pub const SECP256K1_PUBLIC_KEY_LENGTH: usize = 64;

#[repr(transparent)]
#[cfg_attr(feature = "frozen-abi", derive(solana_frozen_abi_macro::AbiExample))]
#[cfg_attr(
    feature = "borsh",
    derive(BorshSerialize, BorshDeserialize, BorshSchema),
    borsh(crate = "borsh")
)]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Secp256k1Pubkey(pub [u8; SECP256K1_PUBLIC_KEY_LENGTH]);

impl Secp256k1Pubkey {
    pub fn new(pubkey_vec: &[u8]) -> Self {
        Self(
            <[u8; SECP256K1_PUBLIC_KEY_LENGTH]>::try_from(<&[u8]>::clone(&pubkey_vec))
                .expect("Slice must be the same length as a Pubkey"),
        )
    }

    pub fn to_bytes(self) -> [u8; 64] {
        self.0
    }
}

#[cfg(any(target_os = "solana", target_arch = "bpf"))]
pub use solana_define_syscall::definitions::sol_secp256k1_recover;

/// Recover the public key from a [secp256k1] ECDSA signature and
/// cryptographically-hashed message.
///
/// [secp256k1]: https://en.bitcoin.it/wiki/Secp256k1
///
/// This function is specifically intended for efficiently implementing
/// Ethereum's [`ecrecover`] builtin contract, for use by Ethereum integrators.
/// It may be useful for other purposes.
///
/// [`ecrecover`]: https://docs.soliditylang.org/en/v0.8.14/units-and-global-variables.html?highlight=ecrecover#mathematical-and-cryptographic-functions
///
/// `hash` is the 32-byte cryptographic hash (typically [`keccak`]) of an
/// arbitrary message, signed by some public key.
///
/// The recovery ID is a value in the range [0, 3] that is generated during
/// signing, and allows the recovery process to be more efficient. Note that the
/// `recovery_id` here does not directly correspond to an Ethereum recovery ID
/// as used in `ecrecover`. This function accepts recovery IDs in the range of
/// [0, 3], while Ethereum's recovery IDs have a value of 27 or 28. To convert
/// an Ethereum recovery ID to a value this function will accept subtract 27
/// from it, checking for underflow. In practice this function will not succeed
/// if given a recovery ID of 2 or 3, as these values represent an
/// "overflowing" signature, and this function returns an error when parsing
/// overflowing signatures.
///
/// [`keccak`]: https://docs.rs/solana-program/latest/solana_program/keccak/
/// [`wrapping_sub`]: https://doc.rust-lang.org/std/primitive.u8.html#method.wrapping_sub
///
/// On success this function returns a [`Secp256k1Pubkey`], a wrapper around a
/// 64-byte secp256k1 public key. This public key corresponds to the secret key
/// that previously signed the message `hash` to produce the provided
/// `signature`.
///
/// While `secp256k1_recover` can be used to verify secp256k1 signatures by
/// comparing the recovered key against an expected key, Solana also provides
/// the [secp256k1 program][sp], which is more flexible, has lower CPU cost, and
/// can validate many signatures at once.
///
/// [sp]: https://docs.rs/solana-program/latest/solana_program/secp256k1_program/
///
/// The `secp256k1_recover` syscall is implemented with the [`libsecp256k1`]
/// crate, but clients may want to use [`k256`] for an up-to-date pure Rust
/// implementation.
///
/// [`libsecp256k1`]: https://docs.rs/libsecp256k1/latest/libsecp256k1
/// [`k256`]: https://docs.rs/k256/latest/k256
///
/// # Hashing messages
///
/// In ECDSA signing and key recovery the signed "message" is always a
/// cryptographic hash, not the original message itself. If not a cryptographic
/// hash, then an adversary can craft signatures that recover to arbitrary
/// public keys. This means the caller of this function generally must hash the
/// original message themselves and not rely on another party to provide the
/// hash.
///
/// Ethereum uses the [`keccak`] hash.
///
/// # Signature malleability
///
/// With the ECDSA signature algorithm it is possible for any party, given a
/// valid signature of some message, to create a second signature that is
/// equally valid. This is known as _signature malleability_. In many cases this
/// is not a concern, but in cases where applications rely on signatures to have
/// a unique representation this can be the source of bugs, potentially with
/// security implications.
///
/// **The solana `secp256k1_recover` function does not prevent signature
/// malleability**. This is in contrast to the Bitcoin secp256k1 library, which
/// does prevent malleability by default. Solana accepts signatures with `S`
/// values that are either in the _high order_ or in the _low order_, and it
/// is trivial to produce one from the other.
///
/// To prevent signature malleability, it is common for secp256k1 signature
/// validators to only accept signatures with low-order `S` values, and reject
/// signatures with high-order `S` values. The following code will accomplish
/// this:
///
/// ```rust
/// # use k256::elliptic_curve::scalar::IsHigh;
/// # use solana_program::program_error::ProgramError;
/// # let signature_bytes = [
/// #     0x83, 0x55, 0x81, 0xDF, 0xB1, 0x02, 0xA7, 0xD2,
/// #     0x2D, 0x33, 0xA4, 0x07, 0xDD, 0x7E, 0xFA, 0x9A,
/// #     0xE8, 0x5F, 0x42, 0x6B, 0x2A, 0x05, 0xBB, 0xFB,
/// #     0xA1, 0xAE, 0x93, 0x84, 0x46, 0x48, 0xE3, 0x35,
/// #     0x74, 0xE1, 0x6D, 0xB4, 0xD0, 0x2D, 0xB2, 0x0B,
/// #     0x3C, 0x89, 0x8D, 0x0A, 0x44, 0xDF, 0x73, 0x9C,
/// #     0x1E, 0xBF, 0x06, 0x8E, 0x8A, 0x9F, 0xA9, 0xC3,
/// #     0xA5, 0xEA, 0x21, 0xAC, 0xED, 0x5B, 0x22, 0x13,
/// # ];
/// let signature = k256::ecdsa::Signature::from_slice(&signature_bytes)
///     .map_err(|_| ProgramError::InvalidArgument)?;
///
/// if bool::from(signature.s().is_high()) {
///     return Err(ProgramError::InvalidArgument);
/// }
/// # Ok::<_, ProgramError>(())
/// ```
///
/// For the most accurate description of signature malleability, and its
/// prevention in secp256k1, refer to comments in [`secp256k1.h`] in the Bitcoin
/// Core secp256k1 library, the documentation of the [OpenZeppelin `recover`
/// method for Solidity][ozr], and [this description of the problem on
/// StackExchange][sxr].
///
/// [`secp256k1.h`]: https://github.com/bitcoin-core/secp256k1/blob/44c2452fd387f7ca604ab42d73746e7d3a44d8a2/include/secp256k1.h
/// [ozr]: https://docs.openzeppelin.com/contracts/2.x/api/cryptography#ECDSA-recover-bytes32-bytes-
/// [sxr]: https://bitcoin.stackexchange.com/questions/81115/if-someone-wanted-to-pretend-to-be-satoshi-by-posting-a-fake-signature-to-defrau/81116#81116
///
/// # Errors
///
/// If `hash` is not 32 bytes in length this function returns
/// [`Secp256k1RecoverError::InvalidHash`], though see notes
/// on SBF-specific behavior below.
///
/// If `recovery_id` is not in the range [0, 3] this function returns
/// [`Secp256k1RecoverError::InvalidRecoveryId`].
///
/// If `signature` is not 64 bytes in length this function returns
/// [`Secp256k1RecoverError::InvalidSignature`], though see notes
/// on SBF-specific behavior below.
///
/// If `signature` represents an "overflowing" signature this function returns
/// [`Secp256k1RecoverError::InvalidSignature`]. Overflowing signatures are
/// non-standard and should not be encountered in practice.
///
/// If `signature` is otherwise invalid this function returns
/// [`Secp256k1RecoverError::InvalidSignature`].
///
/// # SBF-specific behavior
///
/// When calling this function on-chain the caller must verify the correct
/// lengths of `hash` and `signature` beforehand.
///
/// When run on-chain this function will not directly validate the lengths of
/// `hash` and `signature`. It will assume they are the correct lengths and
/// pass their pointers to the runtime, which will interpret them as 32-byte and
/// 64-byte buffers. If the provided slices are too short, the runtime will read
/// invalid data and attempt to interpret it, most likely returning an error,
/// though in some scenarios it may be possible to incorrectly return
/// successfully, or the transaction will abort if the syscall reads data
/// outside of the program's memory space. If the provided slices are too long
/// then they may be used to "smuggle" uninterpreted data.
///
/// # Examples
///
/// This example demonstrates recovering a public key and using it to verify a
/// signature with the `secp256k1_recover` syscall. It has three parts: a Solana
/// program, an RPC client to call the program, and common definitions shared
/// between the two.
///
/// Common definitions:
///
/// ```
/// use borsh::{BorshDeserialize, BorshSerialize};
///
/// #[derive(BorshSerialize, BorshDeserialize, Debug)]
/// # #[borsh(crate = "borsh")]
/// pub struct DemoSecp256k1RecoverInstruction {
///     pub message: Vec<u8>,
///     pub signature: [u8; 64],
///     pub recovery_id: u8,
/// }
/// ```
///
/// The Solana program. Note that it uses `k256` version 0.13.0 to parse
/// the secp256k1 signature to prevent malleability.
///
/// ```rust
/// use k256::elliptic_curve::scalar::IsHigh;
/// use solana_program::{
///     entrypoint::ProgramResult,
///     keccak, msg,
///     program_error::ProgramError,
/// };
/// use solana_secp256k1_recover::secp256k1_recover;
///
/// # pub struct DemoSecp256k1RecoverInstruction {
/// #     pub message: Vec<u8>,
/// #     pub signature: [u8; 64],
/// #     pub recovery_id: u8,
/// # }
///
/// pub fn process_secp256k1_recover(
///     instruction: DemoSecp256k1RecoverInstruction,
///     expected_public_key: [u8; 64],
/// ) -> ProgramResult {
///     // The secp256k1 recovery operation accepts a cryptographically-hashed
///     // message only. Passing it anything else is insecure and allows signatures
///     // to be forged.
///     //
///     // This means that the code calling `secp256k1_recover` must perform the hash
///     // itself, and not assume that data passed to it has been properly hashed.
///     let message_hash = {
///         let mut hasher = keccak::Hasher::default();
///         hasher.hash(&instruction.message);
///         hasher.result()
///     };
///
///     // Reject high-s value signatures to prevent malleability.
///     // Solana does not do this itself.
///     // This may or may not be necessary depending on use case.
///     {
///         let signature = k256::ecdsa::Signature::from_slice(&instruction.signature)
///             .map_err(|_| ProgramError::InvalidArgument)?;
///
///         if bool::from(signature.s().is_high()) {
///             msg!("signature with high-s value");
///             return Err(ProgramError::InvalidArgument);
///         }
///     }
///
///     let recovered_pubkey = secp256k1_recover(
///         message_hash.as_bytes(),
///         instruction.recovery_id,
///         &instruction.signature,
///     )
///     .map_err(|_| ProgramError::InvalidArgument)?;
///
///     // If we're using this function for signature verification then we
///     // need to check the pubkey is an expected value.
///     // Here we are checking the secp256k1 pubkey against a known authorized pubkey.
///     if recovered_pubkey.0 != expected_public_key {
///         return Err(ProgramError::InvalidArgument);
///     }
///
///     Ok(())
/// }
///
/// let secret_key = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
/// let public_key = &secret_key.verifying_key().to_encoded_point(false);
/// let message = b"hello world!";
/// let message_hash = {
///     let mut hasher = keccak::Hasher::default();
///     hasher.hash(message);
///     hasher.result()
/// };
/// let (signature, recovery_id) = secret_key.sign_prehash_recoverable(message_hash.as_bytes()).unwrap();
/// let signature = signature.to_bytes().into();
/// let instruction = DemoSecp256k1RecoverInstruction {
///     message: message.to_vec(),
///     signature,
///     recovery_id: recovery_id.to_byte(),
/// };
/// process_secp256k1_recover(instruction, public_key.as_bytes()[1..65].try_into().unwrap()).unwrap();
/// ```
///
/// The RPC client program:
///
/// ```rust
/// # use solana_program::example_mocks::solana_rpc_client;
/// # use solana_program::example_mocks::solana_sdk;
/// use anyhow::Result;
/// use solana_rpc_client::rpc_client::RpcClient;
/// use solana_sdk::{
///     instruction::Instruction,
///     keccak,
///     pubkey::Pubkey,
///     signature::{Keypair, Signer},
///     transaction::Transaction,
/// };
/// # use borsh::{BorshDeserialize, BorshSerialize};
/// # #[derive(BorshSerialize, BorshDeserialize, Debug)]
/// # #[borsh(crate = "borsh")]
/// # pub struct DemoSecp256k1RecoverInstruction {
/// #     pub message: Vec<u8>,
/// #     pub signature: [u8; 64],
/// #     pub recovery_id: u8,
/// # }
///
/// pub fn demo_secp256k1_recover(
///     payer_keypair: &Keypair,
///     secp256k1_secret_key: &k256::ecdsa::SigningKey,
///     client: &RpcClient,
///     program_keypair: &Keypair,
/// ) -> Result<()> {
///     let message = b"hello world";
///     let message_hash = {
///         let mut hasher = keccak::Hasher::default();
///         hasher.hash(message);
///         hasher.result()
///     };
///
///     let (signature, recovery_id) = secp256k1_secret_key.sign_prehash_recoverable(message_hash.as_bytes()).unwrap();
///
///     let signature = signature.to_bytes().into();
///
///     let instr = DemoSecp256k1RecoverInstruction {
///         message: message.to_vec(),
///         signature,
///         recovery_id: recovery_id.to_byte(),
///     };
///     let instr = Instruction::new_with_borsh(
///         program_keypair.pubkey(),
///         &instr,
///         vec![],
///     );
///
///     let blockhash = client.get_latest_blockhash()?;
///     let tx = Transaction::new_signed_with_payer(
///         &[instr],
///         Some(&payer_keypair.pubkey()),
///         &[payer_keypair],
///         blockhash,
///     );
///
///     client.send_and_confirm_transaction(&tx)?;
///
///     Ok(())
/// }
/// ```
#[cfg_attr(any(target_os = "solana", target_arch = "bpf"), inline(always))]
pub fn secp256k1_recover(
    hash: &[u8],
    recovery_id: u8,
    signature: &[u8],
) -> Result<Secp256k1Pubkey, Secp256k1RecoverError> {
    #[cfg(any(target_os = "solana", target_arch = "bpf"))]
    {
        let mut pubkey_buffer =
            core::mem::MaybeUninit::<[u8; SECP256K1_PUBLIC_KEY_LENGTH]>::uninit();
        let result = unsafe {
            sol_secp256k1_recover(
                hash.as_ptr(),
                recovery_id as u64,
                signature.as_ptr(),
                pubkey_buffer.as_mut_ptr() as *mut u8,
            )
        };

        // SAFETY: This is sound as in our pass case, all 64 bytes of the pubkey are
        // always initialized by sol_secp256k1_recover
        match result {
            0 => Ok(Secp256k1Pubkey(unsafe { pubkey_buffer.assume_init() })),
            error => Err(Secp256k1RecoverError::from(error)),
        }
    }

    #[cfg(not(any(target_os = "solana", target_arch = "bpf")))]
    {
        const HASH_SIZE: usize = 32;
        if hash.len() != HASH_SIZE {
            return Err(Secp256k1RecoverError::InvalidHash);
        }
        let recovery_id = k256::ecdsa::RecoveryId::try_from(recovery_id)
            .map_err(|_| Secp256k1RecoverError::InvalidRecoveryId)?;
        let signature = k256::ecdsa::Signature::from_slice(signature)
            .map_err(|_| Secp256k1RecoverError::InvalidSignature)?;
        let secp256k1_key =
            k256::ecdsa::VerifyingKey::recover_from_prehash(hash, &signature, recovery_id)
                .map_err(|_| Secp256k1RecoverError::InvalidSignature)?;
        Ok(Secp256k1Pubkey::new(
            &secp256k1_key.to_encoded_point(false).as_bytes()[1..65],
        ))
    }
}
