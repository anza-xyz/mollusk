//! Tests for the program cache.

use {
    mollusk_svm::{program::loader_keys, result::Check, Mollusk},
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
};

// The primary test program's no-op instruction.
const NOOP: &[u8] = &[0];

fn process_noop_under_loader(loader_key: &Pubkey) {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mut mollusk = Mollusk::default();
    mollusk.add_program_with_loader(&program_id, "test_program_primary", loader_key);

    mollusk.process_and_validate_instruction(
        &Instruction::new_with_bytes(program_id, NOOP, vec![]),
        &[],
        &[Check::success()],
    );
}

#[test]
fn test_add_program_under_loader_v1() {
    process_noop_under_loader(&loader_keys::LOADER_V1);
}

#[test]
fn test_add_program_under_loader_v2() {
    process_noop_under_loader(&loader_keys::LOADER_V2);
}

#[test]
fn test_add_program_under_loader_v3() {
    process_noop_under_loader(&loader_keys::LOADER_V3);
}
