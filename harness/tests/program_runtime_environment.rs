//! The program runtime environment follows the feature set, and programs
//! already added are reloaded against it when it changes.

use {
    mollusk_svm::{result::Check, Mollusk},
    solana_account::Account,
    solana_instruction::{AccountMeta, Instruction},
    solana_instruction_error::InstructionError,
    solana_pubkey::Pubkey,
};

#[test]
fn test_set_feature_set_reloads_added_programs() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    // The program calls `sol_get_epoch_stake`, which is gated by a feature.
    // It's added under the default feature set, where that syscall is enabled.
    let program_id = Pubkey::new_unique();
    let mut mollusk = Mollusk::new(&program_id, "test_program_epoch_stake");

    let key = Pubkey::new_unique();
    let instruction = Instruction::new_with_bytes(
        program_id,
        &Pubkey::new_unique().to_bytes(),
        vec![AccountMeta::new(key, false)],
    );
    let accounts = [(key, Account::new(1_000, 16, &program_id))];

    mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    // Disabling the syscall reloads the program against an environment without
    // it, so the call fails, as it would on a validator.
    let mut feature_set = *mollusk.feature_set();
    feature_set.enable_get_epoch_stake_syscall = false;
    mollusk.set_feature_set(feature_set);
    mollusk.process_and_validate_instruction(
        &instruction,
        &accounts,
        &[Check::instruction_err(
            InstructionError::ProgramFailedToComplete,
        )],
    );

    // Enabling it again reloads the program with the syscall.
    feature_set.enable_get_epoch_stake_syscall = true;
    mollusk.set_feature_set(feature_set);
    mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);
}

#[test]
#[should_panic(
    expected = "Failed to load program ELF with the current program runtime environment"
)]
fn test_set_feature_set_program_that_no_longer_verifies() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mut mollusk = Mollusk::default();
    mollusk.add_program(&program_id, "test_program_cpi_target_v0");

    // Disabling SBPF v0 execution means the ELF no longer loads when it's
    // reloaded, which panics like adding it under that feature set would.
    let mut feature_set = *mollusk.feature_set();
    feature_set.disable_sbpf_v0_execution = true;
    feature_set.reenable_sbpf_v0_execution = false;
    mollusk.set_feature_set(feature_set);
}
