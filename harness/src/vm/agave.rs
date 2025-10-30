//! The SBPF virtual machine used in Anza's Agave validator.

#[cfg(feature = "invocation-inspect-callback")]
use crate::InvocationInspectCallback;
use {
    super::{SolanaVM, SolanaVMContext, SolanaVMInstruction, SolanaVMTrace},
    solana_instruction_error::InstructionError,
    solana_program_runtime::invoke_context::InvokeContext,
};

/// The SBPF virtual machine used in Anza's Agave validator.
pub struct AgaveVM {}

impl SolanaVM for AgaveVM {
    fn process_instruction(
        context: SolanaVMContext,
        instruction: SolanaVMInstruction,
        trace: SolanaVMTrace,
        #[cfg(feature = "invocation-inspect-callback")]
        invocation_inspect_callback: &dyn InvocationInspectCallback,
    ) -> Result<(), InstructionError> {
        let mut invoke_context = InvokeContext::new(
            context.transaction_context,
            context.program_cache,
            context.environment_config,
            trace.log_collector,
            context.compute_budget.to_budget(),
            context.compute_budget.to_cost(),
        );

        // Configure the next instruction frame for this invocation.
        invoke_context
            .transaction_context
            .configure_next_instruction_for_tests(
                instruction.program_id_index,
                instruction.accounts.clone(),
                instruction.data,
            )
            .expect("failed to configure next instruction");

        let program_id = invoke_context
            .transaction_context
            .get_key_of_account_at_index(instruction.program_id_index)
            .cloned()?;

        #[cfg(feature = "invocation-inspect-callback")]
        invocation_inspect_callback.before_invocation(
            &program_id,
            instruction.data,
            &instruction.accounts,
            &invoke_context,
        );

        let result = if invoke_context.is_precompile(&program_id) {
            invoke_context.process_precompile(
                &program_id,
                instruction.data,
                std::iter::once(instruction.data),
            )
        } else {
            invoke_context.process_instruction(trace.compute_units_consumed, trace.execute_timings)
        };

        #[cfg(feature = "invocation-inspect-callback")]
        invocation_inspect_callback.after_invocation(&invoke_context);

        result
    }
}
