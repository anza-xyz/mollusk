//! Solana account addresses.
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(feature = "frozen-abi", feature(min_specialization))]
#![allow(clippy::arithmetic_side_effects)]

// If target_os = "solana", then this panics so there are no dependencies.
// When target_os != "solana", this should be opt-in so users
// don't need the curve25519 dependency.
#[cfg(any(target_os = "solana", feature = "curve25519"))]
pub use solana_address::bytes_are_curve_point;
#[cfg(target_os = "solana")]
pub use solana_address::syscalls;
pub use solana_address::{
    address as pubkey, declare_deprecated_id, declare_id,
    error::{AddressError as PubkeyError, ParseAddressError as ParsePubkeyError},
    Address as Pubkey, ADDRESS_BYTES as PUBKEY_BYTES, MAX_SEEDS, MAX_SEED_LEN,
};
#[cfg(all(feature = "rand", not(target_os = "solana")))]
pub use solana_address::{
    AddressHasher as PubkeyHasher, AddressHasherBuilder as PubkeyHasherBuilder,
};

/// New random `Pubkey` for tests and benchmarks.
#[cfg(all(feature = "rand", not(target_os = "solana")))]
pub fn new_rand() -> Pubkey {
    Pubkey::from(rand::random::<[u8; PUBKEY_BYTES]>())
}
