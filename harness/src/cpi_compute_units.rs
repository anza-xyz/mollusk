//! Per-CPI compute unit tracking.
//!
//! The CPI syscalls are replaced with wrappers that call Agave's own
//! implementations and record the compute units consumed by each call.

use {
    mollusk_svm_result::types::CpiComputeUnits,
    solana_program_runtime::{
        invoke_context::InvokeContext,
        loaded_programs::ProgramRuntimeEnvironment,
        solana_sbpf::{
            declare_builtin_function,
            program::{BuiltinFunctionDefinition, BuiltinProgram},
            vm::ContextObject,
        },
    },
    solana_syscalls::{SyscallInvokeSignedC, SyscallInvokeSignedRust},
    solana_transaction_context::transaction::TransactionContext,
    std::cell::RefCell,
};

type Error = Box<dyn std::error::Error>;

thread_local! {
    static RECORDS: RefCell<Vec<(usize, u64)>> = const { RefCell::new(Vec::new()) };
}

fn measure<'a, 'b>(
    invoke_context: &mut InvokeContext<'a, 'b>,
    cpi: impl FnOnce(&mut InvokeContext<'a, 'b>) -> Result<u64, Error>,
) -> Result<u64, Error> {
    // The callee is pushed at the current trace length.
    let index_in_trace = invoke_context
        .transaction_context
        .get_instruction_trace_length();
    let remaining = invoke_context.get_remaining();
    let result = cpi(invoke_context);
    // A CPI that fails before invoking the callee has no trace entry.
    if invoke_context
        .transaction_context
        .get_instruction_trace_length()
        > index_in_trace
    {
        let consumed = remaining.saturating_sub(invoke_context.get_remaining());
        RECORDS.with_borrow_mut(|records| records.push((index_in_trace, consumed)));
    }
    result
}

declare_builtin_function!(
    SyscallInvokeSignedRustMetered,
    fn rust(
        invoke_context: &mut InvokeContext<'_, '_>,
        instruction_addr: u64,
        account_infos_addr: u64,
        account_infos_len: u64,
        signers_seeds_addr: u64,
        signers_seeds_len: u64,
    ) -> Result<u64, Error> {
        measure(invoke_context, |invoke_context| {
            SyscallInvokeSignedRust::rust(
                invoke_context,
                instruction_addr,
                account_infos_addr,
                account_infos_len,
                signers_seeds_addr,
                signers_seeds_len,
            )
        })
    }
);

declare_builtin_function!(
    SyscallInvokeSignedCMetered,
    fn rust(
        invoke_context: &mut InvokeContext<'_, '_>,
        instruction_addr: u64,
        account_infos_addr: u64,
        account_infos_len: u64,
        signers_seeds_addr: u64,
        signers_seeds_len: u64,
    ) -> Result<u64, Error> {
        measure(invoke_context, |invoke_context| {
            SyscallInvokeSignedC::rust(
                invoke_context,
                instruction_addr,
                account_infos_addr,
                account_infos_len,
                signers_seeds_addr,
                signers_seeds_len,
            )
        })
    }
);

pub(crate) fn instrument(
    mut loader: BuiltinProgram<InvokeContext<'static, 'static>>,
) -> ProgramRuntimeEnvironment {
    loader.unregister_function("sol_invoke_signed_c");
    loader.unregister_function("sol_invoke_signed_rust");
    SyscallInvokeSignedCMetered::register(&mut loader, "sol_invoke_signed_c").unwrap();
    SyscallInvokeSignedRustMetered::register(&mut loader, "sol_invoke_signed_rust").unwrap();
    ProgramRuntimeEnvironment::from(loader)
}

pub(crate) fn clear() {
    RECORDS.with_borrow_mut(Vec::clear);
}

pub(crate) fn take(transaction_context: &TransactionContext) -> Vec<CpiComputeUnits> {
    let mut records = RECORDS.with_borrow_mut(std::mem::take);
    // Records are pushed as CPIs return, so nested CPIs come before their
    // callers. Trace order is invocation order.
    records.sort_unstable_by_key(|(index_in_trace, _)| *index_in_trace);
    records
        .into_iter()
        .map(|(index_in_trace, compute_units_consumed)| {
            let instruction_context = transaction_context
                .get_instruction_context_at_index_in_trace(index_in_trace)
                .unwrap();
            CpiComputeUnits {
                stack_height: instruction_context.get_stack_height(),
                program_id: *instruction_context.get_program_key().unwrap(),
                compute_units_consumed,
            }
        })
        .collect()
}
