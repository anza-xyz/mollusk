//! Tests for loading SBPF ELFs.

use {
    mollusk_svm::{program::ProgramCache, Mollusk},
    solana_pubkey::Pubkey,
};

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
fn test_sbpf_v3_feature_is_active_by_default() {
    let mollusk = Mollusk::default();
    assert!(mollusk.feature_set.enable_sbpf_v3_deployment_and_execution);
}

#[test]
fn test_sbpf_v3_elf_loads_with_feature_enabled() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mut mollusk = Mollusk::default();
    mollusk.add_program(&program_id, "test_program_cpi_target_v3");
}

#[test]
#[should_panic(expected = "called `Result::unwrap()` on an `Err` value: UnsupportedSBPFVersion")]
fn test_sbpf_v3_elf_fails_without_feature() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let mut mollusk = Mollusk::default();
    mollusk.feature_set.enable_sbpf_v3_deployment_and_execution = false;

    // Rebuild the program cache with the updated feature set so that
    // the runtime environment no longer includes V3 in
    // `enabled_sbpf_versions`.
    mollusk.program_cache = ProgramCache::new(&mollusk.feature_set, &mollusk.compute_budget, false);

    let program_id = Pubkey::new_unique();
    let elf = mollusk_svm::file::load_program_elf("test_program_cpi_target_v3");

    // Loading a V3 ELF should fail because V3 is not in the
    // enabled range.
    mollusk.program_cache.add_program(
        &program_id,
        &solana_sdk_ids::bpf_loader_upgradeable::ID,
        &elf,
    );
}
