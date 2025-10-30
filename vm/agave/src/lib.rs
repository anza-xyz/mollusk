//! The SBPF virtual machine used in Anza's Agave validator.

pub mod compile_accounts;
pub mod precompile_keys;

#[cfg(feature = "invocation-inspect-callback")]
use mollusk_svm_vm::InvocationInspectCallback;
use {
    compile_accounts::{compile_accounts, CompiledAccounts},
    mollusk_svm_error::error::{MolluskError, MolluskPanic},
    mollusk_svm_result::InstructionResult,
    mollusk_svm_vm::{SolanaVM, SolanaVMContext},
    solana_account::Account,
    solana_instruction::Instruction,
    solana_program_runtime::invoke_context::InvokeContext,
    solana_pubkey::Pubkey,
    solana_svm_timings::ExecuteTimings,
    solana_transaction_context::TransactionContext,
};

/// The SBPF virtual machine used in Anza's Agave validator.
pub struct AgaveVM;

impl SolanaVM for AgaveVM {
    fn process_instruction(
        context: SolanaVMContext,
        instruction: &Instruction,
        accounts: &[(Pubkey, Account)],
        #[cfg(feature = "invocation-inspect-callback")]
        invocation_inspect_callback: &dyn InvocationInspectCallback,
    ) -> InstructionResult {
        let mut compute_units_consumed = 0;
        let mut timings = ExecuteTimings::default();

        let loader_key = if crate::precompile_keys::is_precompile(&instruction.program_id) {
            solana_sdk_ids::native_loader::id()
        } else {
            context
                .program_cache
                .find(&instruction.program_id)
                .or_panic_with(MolluskError::ProgramNotCached(&instruction.program_id))
                .account_owner()
        };

        let CompiledAccounts {
            program_id_index,
            instruction_accounts,
            transaction_accounts,
        } = compile_accounts(instruction, accounts, loader_key);

        let mut transaction_context = TransactionContext::new(
            transaction_accounts,
            context.rent,
            context.compute_budget.max_instruction_stack_depth,
            context.compute_budget.max_instruction_trace_length,
        );

        let invoke_result = {
            let mut invoke_context = InvokeContext::new(
                &mut transaction_context,
                context.program_cache,
                context.environment_config,
                context.log_collector,
                context.compute_budget.to_budget(),
                context.compute_budget.to_cost(),
            );

            // Configure the next instruction frame for this invocation.
            invoke_context
                .transaction_context
                .configure_next_instruction_for_tests(
                    program_id_index,
                    instruction_accounts.clone(),
                    &instruction.data,
                )
                .expect("failed to configure next instruction");

            let program_id = invoke_context
                .transaction_context
                .get_key_of_account_at_index(program_id_index)
                .cloned()
                .expect("failed to get program id");

            #[cfg(feature = "invocation-inspect-callback")]
            invocation_inspect_callback.before_invocation(
                &program_id,
                &instruction.data,
                &instruction_accounts,
                &invoke_context,
            );

            let result = if invoke_context.is_precompile(&program_id) {
                invoke_context.process_precompile(
                    &program_id,
                    &instruction.data,
                    std::iter::once(&instruction.data[..]),
                )
            } else {
                invoke_context.process_instruction(&mut compute_units_consumed, &mut timings)
            };

            #[cfg(feature = "invocation-inspect-callback")]
            invocation_inspect_callback.after_invocation(&invoke_context);

            result
        };

        let return_data = transaction_context.get_return_data().1.to_vec();

        let resulting_accounts: Vec<(solana_pubkey::Pubkey, Account)> = if invoke_result.is_ok() {
            accounts
                .iter()
                .map(|(pubkey, account)| {
                    transaction_context
                        .find_index_of_account(pubkey)
                        .map(|index| {
                            let resulting_account = transaction_context
                                .accounts()
                                .try_borrow(index)
                                .unwrap()
                                .clone()
                                .into();
                            (*pubkey, resulting_account)
                        })
                        .unwrap_or((*pubkey, account.clone()))
                })
                .collect()
        } else {
            accounts.to_vec()
        };

        InstructionResult {
            compute_units_consumed,
            execution_time: timings.details.execute_us.0,
            program_result: invoke_result.clone().into(),
            raw_result: invoke_result,
            return_data,
            resulting_accounts,
        }
    }
}
