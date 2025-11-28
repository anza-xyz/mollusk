#[cfg(feature = "register-tracing")]
#[test]
fn test_custom_register_tracing_callback() {
    use {
        mollusk_svm::{InvocationInspectCallback, Mollusk},
        solana_account::Account,
        solana_instruction::{AccountMeta, Instruction},
        solana_program_runtime::invoke_context::{Executable, InvokeContext, RegisterTrace},
        solana_pubkey::Pubkey,
        solana_transaction_context::{InstructionAccount, InstructionContext},
        std::{cell::RefCell, collections::HashMap, rc::Rc},
    };

    struct TracingData {
        program_id: Pubkey,
        executed_jump_instructions_count: usize,
    }

    struct CustomRegisterTracingCallback {
        tracing_data: Rc<RefCell<HashMap<Pubkey, TracingData>>>,
    }

    impl CustomRegisterTracingCallback {
        fn handler(
            &self,
            instruction_context: InstructionContext,
            executable: &Executable,
            register_trace: RegisterTrace,
        ) -> Result<(), Box<dyn std::error::Error + 'static>> {
            let mut tracing_data = self.tracing_data.try_borrow_mut()?;

            let program_id = instruction_context.get_program_key().unwrap();
            let (_vm_addr, program) = executable.get_text_bytes();
            let executed_jump_instructions_count = register_trace
                .iter()
                .map(|registers| {
                    (
                        registers,
                        solana_program_runtime::solana_sbpf::ebpf::get_insn_unchecked(
                            program,
                            registers[11] as usize,
                        ),
                    )
                })
                .filter(|(_registers, insn)| {
                    insn.opc & 7 == solana_program_runtime::solana_sbpf::ebpf::BPF_JMP
                        && insn.opc != solana_program_runtime::solana_sbpf::ebpf::BPF_JA
                })
                .count();
            let entry = tracing_data.entry(*program_id).or_insert(TracingData {
                program_id: *program_id,
                executed_jump_instructions_count: 0,
            });
            entry.executed_jump_instructions_count = entry
                .executed_jump_instructions_count
                .saturating_add(executed_jump_instructions_count);

            Ok(())
        }
    }

    impl InvocationInspectCallback for CustomRegisterTracingCallback {
        fn before_invocation(
            &self,
            _: &Pubkey,
            _: &[u8],
            _: &[InstructionAccount],
            _: &InvokeContext,
        ) {
        }

        fn after_invocation(&self, invoke_context: &InvokeContext) {
            invoke_context.iterate_vm_traces(
                &|instruction_context: InstructionContext,
                  executable: &Executable,
                  register_trace: RegisterTrace| {
                    if let Err(e) = self.handler(instruction_context, executable, register_trace) {
                        eprintln!("Error collecting the register tracing: {}", e);
                    }
                },
            );
        }
    }

    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let payer_pk = Pubkey::new_unique();
    let mut mollusk = Mollusk::new(&program_id, "test_program_primary");

    // Phase 1 - basic register tracing test.

    // Have a custom register tracing handler counting the total number of executed
    // jump instructions per program_id.
    let tracing_data = Rc::new(RefCell::new(HashMap::<Pubkey, TracingData>::new()));
    mollusk.invocation_inspect_callback = Box::new(CustomRegisterTracingCallback {
        tracing_data: Rc::clone(&tracing_data),
    });

    let (system_program_id, system_account) =
        mollusk_svm::program::keyed_account_for_system_program();

    let ix_data = [0, 0];
    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_data,
        vec![
            AccountMeta::new(payer_pk, true),
            AccountMeta::new(system_program_id, false),
        ],
    );

    let base_lamports = 100_000_000u64;
    let accounts = vec![
        (payer_pk, Account::new(base_lamports, 0, &system_program_id)),
        (system_program_id, system_account),
    ];

    // Execute the instruction.
    let _ = mollusk.process_instruction(&instruction, &accounts);

    let executed_jump_instruction_count_from_phase1;
    // Let's check the outcome of the custom register tracing callback.
    {
        assert_eq!(tracing_data.borrow().len(), 1);
        let td = tracing_data.borrow();
        let collected_data = td.get(&program_id).unwrap();

        // Check it's the program_id only on our list.
        assert_eq!(collected_data.program_id, program_id);
        // Check the number of executed jump class instructions is greater than 0.
        assert!(collected_data.executed_jump_instructions_count > 0);

        // Store this value for a later comparison.
        executed_jump_instruction_count_from_phase1 =
            collected_data.executed_jump_instructions_count;
    }

    // Phase 2 - check we can stop register tracing for this instance of Mollusk.
    {
        // Clear the tracing data collected so far.
        {
            let mut td = tracing_data.borrow_mut();
            td.clear();
        }

        mollusk.enable_register_tracing = false;

        // Execute the same instruction again.
        let _ = mollusk.process_instruction(&instruction, &accounts);

        let td = tracing_data.borrow();
        // We expect it to be empty!
        assert!(td.is_empty());
    }

    // Phase 3 - check we can have register tracing back for this instance of
    // Mollusk.
    {
        mollusk.enable_register_tracing = true;

        // Execute the same instruction again.
        let _ = mollusk.process_instruction(&instruction, &accounts);

        let td = tracing_data.borrow();
        let collected_data = td.get(&program_id).unwrap();

        // Check again it's the program_id only on our list.
        assert_eq!(collected_data.program_id, program_id);
        // Check the number of executed jump instructions is the same as we did in
        // phase 1 of this test.
        assert!(
            collected_data.executed_jump_instructions_count
                == executed_jump_instruction_count_from_phase1
        );
    }
}
