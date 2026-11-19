use {
    mollusk_svm::{result::Check, Mollusk},
    solana_instruction::Instruction,
    solana_program_runtime::{
        invoke_context::InvokeContext,
        solana_sbpf::{declare_builtin_function, program::BuiltinFunctionDefinition},
    },
    solana_pubkey::Pubkey,
};

declare_builtin_function!(
    /// A custom syscall to burn CUs.
    SyscallBurnCus,
    fn rust(
        invoke_context: &mut InvokeContext<'_, '_>,
        to_burn: u64,
        _arg2: u64,
        _arg3: u64,
        _arg4: u64,
        _arg5: u64,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        invoke_context.consume_checked(to_burn)?;
        Ok(0)
    }
);

fn instruction_burn_cus(program_id: &Pubkey, to_burn: u64) -> Instruction {
    Instruction::new_with_bytes(*program_id, &to_burn.to_le_bytes(), vec![])
}

#[test]
fn test_custom_syscall() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();

    let mollusk = {
        let mut mollusk = Mollusk::default();
        mollusk
            .program_cache
            .register_builtin_function("sol_burn_cus", SyscallBurnCus::register)
            .unwrap();
        mollusk.add_program_with_loader(
            &program_id,
            "test_program_custom_syscall",
            &mollusk_svm::program::loader_keys::LOADER_V3,
        );
        mollusk
    };

    let base_cus = mollusk
        .process_and_validate_instruction(
            &instruction_burn_cus(&program_id, 0),
            &[],
            &[Check::success()],
        )
        .compute_units_consumed;

    for to_burn in [100, 1_000, 10_000] {
        mollusk.process_and_validate_instruction(
            &instruction_burn_cus(&program_id, to_burn),
            &[],
            &[Check::success(), Check::compute_units(base_cus + to_burn)],
        );
    }
}

#[test]
fn test_custom_syscall_survives_environment_rebuild() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();

    let mut mollusk = Mollusk::default();
    mollusk
        .program_cache
        .register_builtin_function("sol_burn_cus", SyscallBurnCus::register)
        .unwrap();
    mollusk.add_program_with_loader(
        &program_id,
        "test_program_custom_syscall",
        &mollusk_svm::program::loader_keys::LOADER_V3,
    );

    // Setting the feature set rebuilds the environment and reloads the
    // program. The custom syscall must be registered again, or the program
    // would no longer verify.
    let feature_set = *mollusk.feature_set();
    mollusk.set_feature_set(feature_set);

    let base_cus = mollusk
        .process_and_validate_instruction(
            &instruction_burn_cus(&program_id, 0),
            &[],
            &[Check::success()],
        )
        .compute_units_consumed;
    mollusk.process_and_validate_instruction(
        &instruction_burn_cus(&program_id, 1_000),
        &[],
        &[Check::success(), Check::compute_units(base_cus + 1_000)],
    );
}

#[test]
fn test_custom_syscall_registered_after_adding_program() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();

    // The program is added before the syscall it calls is registered.
    // Registering it reloads the program, so the call resolves.
    let mut mollusk = Mollusk::default();
    mollusk.add_program_with_loader(
        &program_id,
        "test_program_custom_syscall",
        &mollusk_svm::program::loader_keys::LOADER_V3,
    );
    mollusk
        .program_cache
        .register_builtin_function("sol_burn_cus", SyscallBurnCus::register)
        .unwrap();

    let base_cus = mollusk
        .process_and_validate_instruction(
            &instruction_burn_cus(&program_id, 0),
            &[],
            &[Check::success()],
        )
        .compute_units_consumed;
    mollusk.process_and_validate_instruction(
        &instruction_burn_cus(&program_id, 1_000),
        &[],
        &[Check::success(), Check::compute_units(base_cus + 1_000)],
    );
}
