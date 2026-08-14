//! Callback for inspecting the invoke context around program invocations.

use {
    crate::Mollusk, solana_program_runtime::invoke_context::InvokeContext, solana_pubkey::Pubkey,
    solana_transaction_context::instruction_accounts::InstructionAccount,
};

pub trait InvocationInspectCallback {
    fn before_invocation(
        &self,
        mollusk: &Mollusk,
        program_id: &Pubkey,
        instruction_data: &[u8],
        instruction_accounts: &[InstructionAccount],
        invoke_context: &mut InvokeContext,
        register_tracing_enabled: bool,
    );

    fn after_invocation(
        &self,
        mollusk: &Mollusk,
        invoke_context: &InvokeContext,
        register_tracing_enabled: bool,
    );
}

pub struct EmptyInvocationInspectCallback;

impl InvocationInspectCallback for EmptyInvocationInspectCallback {
    fn before_invocation(
        &self,
        _: &Mollusk,
        _: &Pubkey,
        _: &[u8],
        _: &[InstructionAccount],
        _: &mut InvokeContext,
        _register_tracing_enabled: bool,
    ) {
    }

    fn after_invocation(&self, _: &Mollusk, _: &InvokeContext, _register_tracing_enabled: bool) {}
}
