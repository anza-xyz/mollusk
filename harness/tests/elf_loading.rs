//! Tests for loading SBPF ELFs.

use {mollusk_svm::Mollusk, solana_pubkey::Pubkey};

#[test]
fn test_sbpf_v0_elf_loads() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mut mollusk = Mollusk::default();
    mollusk.add_program(&program_id, "test_program_cpi_target_v0");
}

#[test]
fn test_sbpf_v1_elf_loads() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mut mollusk = Mollusk::default();
    mollusk.add_program(&program_id, "test_program_cpi_target_v1");
}

#[test]
fn test_sbpf_v2_elf_loads() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mut mollusk = Mollusk::default();
    mollusk.add_program(&program_id, "test_program_cpi_target_v2");
}

#[test]
fn test_sbpf_v3_elf_loads() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mut mollusk = Mollusk::default();
    mollusk.add_program(&program_id, "test_program_cpi_target_v3");
}
