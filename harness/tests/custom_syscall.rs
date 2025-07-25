use {
    mollusk_svm::{result::Check, Mollusk},
    solana_instruction::Instruction,
    solana_program_runtime::{
        invoke_context::InvokeContext,
        solana_sbpf::{declare_builtin_function, memory_region::MemoryMapping},
    },
    solana_pubkey::Pubkey,
};

declare_builtin_function!(
    /// A custom syscall to burn CUs.
    SyscallBurnCus,
    fn rust(
        invoke_context: &mut InvokeContext,
        to_burn: u64,
        _arg2: u64,
        _arg3: u64,
        _arg4: u64,
        _arg5: u64,
        _memory_mapping: &mut MemoryMapping,
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
            .program_runtime_environment
            .register_function("sol_burn_cus", SyscallBurnCus::vm)
            .unwrap();
        mollusk.add_program(
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
