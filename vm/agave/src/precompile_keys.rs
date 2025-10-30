//! Precompile keys and utilities.

use solana_pubkey::Pubkey;
#[cfg(feature = "precompiles")]
pub use solana_sdk_ids::{
    ed25519_program::ID as ED25519_PROGRAM, secp256k1_program::ID as SECP256K1_PROGRAM,
    secp256r1_program::ID as SECP256R1_PROGRAM,
};

#[cfg(feature = "precompiles")]
pub fn is_precompile(program_id: &Pubkey) -> bool {
    matches!(
        *program_id,
        ED25519_PROGRAM | SECP256K1_PROGRAM | SECP256R1_PROGRAM
    )
}

#[cfg(not(feature = "precompiles"))]
pub fn is_precompile(_program_id: &Pubkey) -> bool {
    false
}
