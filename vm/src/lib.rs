//! Virtual Machine API for using Mollusk with custom VMs.

use {
    agave_feature_set::FeatureSet,
    mollusk_svm_result::InstructionResult,
    solana_account::Account,
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_instruction::Instruction,
    solana_program_runtime::{loaded_programs::ProgramCacheForTxBatch, sysvar_cache::SysvarCache},
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    solana_svm_log_collector::LogCollector,
    std::{cell::RefCell, collections::HashMap, rc::Rc},
};
#[cfg(feature = "invocation-inspect-callback")]
use {
    solana_program_runtime::invoke_context::InvokeContext,
    solana_transaction_context::InstructionAccount,
};

/// Context required to process a Solana instruction in a VM.
pub struct SolanaVMContext<'a> {
    pub program_cache: &'a mut ProgramCacheForTxBatch,
    pub compute_budget: ComputeBudget,
    pub feature_set: &'a FeatureSet,
    pub epoch_stake: &'a HashMap<Pubkey, u64>,
    pub sysvar_cache: &'a SysvarCache,
    pub log_collector: Option<Rc<RefCell<LogCollector>>>,
    pub rent: Rent,
}

#[cfg(feature = "invocation-inspect-callback")]
pub trait InvocationInspectCallback {
    fn before_invocation(
        &self,
        program_id: &Pubkey,
        instruction_data: &[u8],
        instruction_accounts: &[InstructionAccount],
        invoke_context: &InvokeContext,
    );

    fn after_invocation(&self, invoke_context: &InvokeContext);
}

#[cfg(feature = "invocation-inspect-callback")]
pub struct EmptyInvocationInspectCallback;

#[cfg(feature = "invocation-inspect-callback")]
impl InvocationInspectCallback for EmptyInvocationInspectCallback {
    fn before_invocation(&self, _: &Pubkey, _: &[u8], _: &[InstructionAccount], _: &InvokeContext) {
    }

    fn after_invocation(&self, _: &InvokeContext) {}
}

/// A virtual machine compatible with the Solana calling convention.
pub trait SolanaVM {
    /// Process a Solana instruction.
    fn process_instruction(
        context: SolanaVMContext,
        instruction: &Instruction,
        accounts: &[(Pubkey, Account)],
        #[cfg(feature = "invocation-inspect-callback")]
        invocation_inspect_callback: &dyn InvocationInspectCallback,
    ) -> InstructionResult;
}
