//! Virtual Machine API for using Mollusk with custom VMs.

pub mod agave;

#[cfg(feature = "invocation-inspect-callback")]
use crate::InvocationInspectCallback;
use {
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_instruction_error::InstructionError,
    solana_program_runtime::{
        invoke_context::EnvironmentConfig, loaded_programs::ProgramCacheForTxBatch,
    },
    solana_svm_log_collector::LogCollector,
    solana_svm_timings::ExecuteTimings,
    solana_transaction_context::{InstructionAccount, TransactionContext},
    std::{cell::RefCell, rc::Rc},
};

/// Context required to process a Solana instruction in a VM.
pub struct SolanaVMContext<'a> {
    pub transaction_context: &'a mut TransactionContext,
    pub program_cache: &'a mut ProgramCacheForTxBatch,
    pub compute_budget: ComputeBudget,
    pub environment_config: EnvironmentConfig<'a>,
}

/// A Solana instruction to be processed by a VM.
pub struct SolanaVMInstruction<'a> {
    pub program_id_index: u16,
    pub accounts: Vec<InstructionAccount>,
    pub data: &'a [u8],
}

/// Trace information about a Solana VM instruction invocation.
pub struct SolanaVMTrace<'a> {
    pub compute_units_consumed: &'a mut u64,
    pub execute_timings: &'a mut ExecuteTimings,
    pub log_collector: Option<Rc<RefCell<LogCollector>>>,
}

/// A virtual machine compatible with the Solana calling convention.
pub trait SolanaVM {
    /// Process a Solana instruction.
    fn process_instruction(
        context: SolanaVMContext,
        instruction: SolanaVMInstruction,
        trace: SolanaVMTrace,
        #[cfg(feature = "invocation-inspect-callback")]
        invocation_inspect_callback: &dyn InvocationInspectCallback,
    ) -> Result<(), InstructionError>;
}
