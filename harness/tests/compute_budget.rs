use {
    mollusk_svm::{program::Builtin, result::Check, Mollusk},
    solana_account::Account,
    solana_instruction::Instruction,
    solana_instruction_error::InstructionError,
    solana_program_runtime::{
        declare_process_instruction, solana_sbpf::program::BuiltinFunctionDefinition,
    },
    solana_pubkey::Pubkey,
    solana_system_program::system_processor::DEFAULT_COMPUTE_UNITS,
};

// Exceeds the default limit of 200k, which needs a custom builtin to reach.
const EXPENSIVE_BUILTIN_COMPUTE_UNITS: u64 = 300_000;

declare_process_instruction!(
    ExpensiveBuiltin,
    EXPENSIVE_BUILTIN_COMPUTE_UNITS,
    |_invoke_context| { Ok(()) }
);

#[test]
fn test_compute_unit_limit_below_default() {
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let mut mollusk = Mollusk::default();

    let instruction = solana_system_interface::instruction::transfer(&sender, &recipient, 42_000);
    let accounts = [
        (
            sender,
            Account::empty(100_000_000, &solana_sdk_ids::system_program::id()),
        ),
        (
            recipient,
            Account::empty(100_000_000, &solana_sdk_ids::system_program::id()),
        ),
    ];

    // Fail - One compute unit short.
    mollusk.compute_budget.compute_unit_limit = DEFAULT_COMPUTE_UNITS - 1;
    mollusk.process_and_validate_instruction(
        &instruction,
        &accounts,
        &[Check::instruction_err(
            InstructionError::ComputationalBudgetExceeded,
        )],
    );

    // Success
    mollusk.compute_budget.compute_unit_limit = DEFAULT_COMPUTE_UNITS;
    mollusk.process_and_validate_instruction(
        &instruction,
        &accounts,
        &[
            Check::success(),
            Check::compute_units(DEFAULT_COMPUTE_UNITS),
        ],
    );
}

#[test]
fn test_compute_unit_limit_above_default() {
    let program_id = Pubkey::new_unique();

    let mut mollusk = Mollusk::default();
    mollusk.program_cache.add_builtin(Builtin {
        program_id,
        name: "expensive_builtin",
        register_fn: ExpensiveBuiltin::register,
    });

    let instruction = Instruction::new_with_bytes(program_id, &[], vec![]);

    // Fail - One compute unit short.
    mollusk.compute_budget.compute_unit_limit = EXPENSIVE_BUILTIN_COMPUTE_UNITS - 1;
    mollusk.process_and_validate_instruction(
        &instruction,
        &[],
        &[Check::instruction_err(
            InstructionError::ComputationalBudgetExceeded,
        )],
    );

    // Success
    mollusk.compute_budget.compute_unit_limit = EXPENSIVE_BUILTIN_COMPUTE_UNITS;
    mollusk.process_and_validate_instruction(
        &instruction,
        &[],
        &[
            Check::success(),
            Check::compute_units(EXPENSIVE_BUILTIN_COMPUTE_UNITS),
        ],
    );
}
