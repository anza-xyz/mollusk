use {
    mollusk::{
        result::{InstructionCheck, ProgramResult},
        Mollusk,
    },
    solana_sdk::{instruction::Instruction, program_error::ProgramError, pubkey::Pubkey},
};

#[test]
fn test_set_return_data() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();

    let instruction = Instruction::new_with_bytes(program_id, &[1], vec![]);
    let checks = vec![
        InstructionCheck::program_result(ProgramResult::Success),
        InstructionCheck::compute_units_consumed(143),
    ];

    let mollusk = Mollusk::new(&program_id, "test_program");

    mollusk.process_and_validate_instruction(&instruction, vec![], &checks);
}

#[test]
fn test_fail_empty_input() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();

    let instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
    let checks = vec![
        InstructionCheck::program_result(ProgramResult::Failure(
            ProgramError::InvalidInstructionData,
        )),
        InstructionCheck::compute_units_consumed(55),
    ];

    let mollusk = Mollusk::new(&program_id, "test_program");

    mollusk.process_and_validate_instruction(&instruction, vec![], &checks);
}
