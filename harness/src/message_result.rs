//! The intermediate result of executing a transaction message.

use {
    mollusk_svm_result::types::TransactionProgramResult,
    solana_instruction_error::InstructionError, solana_program_error::ProgramError,
    solana_transaction_error::TransactionError,
};
#[cfg(feature = "inner-instructions")]
use {solana_message::SanitizedMessage, solana_transaction_status_client_types::InnerInstruction};

pub(crate) struct MessageResult {
    /// The number of compute units consumed by the transaction.
    pub compute_units_consumed: u64,
    /// The time taken to execute the transaction, in microseconds.
    pub execution_time: u64,
    /// The raw result of the transaction's execution.
    pub raw_result: Result<(), TransactionError>,
    /// The return data produced by the transaction, if any.
    pub return_data: Vec<u8>,
    /// Inner instructions (CPIs) invoked during the transaction execution.
    ///
    /// Each entry represents a cross-program invocation made by the program,
    /// including the invoked instruction and the stack height at which it
    /// was called.
    #[cfg(feature = "inner-instructions")]
    pub inner_instructions: Vec<Vec<InnerInstruction>>,
    /// The compiled message used to execute the transaction.
    ///
    /// This can be used to map account indices in inner instructions back to
    /// their corresponding pubkeys via `message.account_keys()`.
    ///
    /// This is `None` when the result is loaded from a fuzz fixture, since
    /// fixtures don't contain the compiled message.
    #[cfg(feature = "inner-instructions")]
    pub message: Option<SanitizedMessage>,
}

impl MessageResult {
    pub(crate) fn extract_ix_err(txn_err: TransactionError) -> InstructionError {
        match txn_err {
            TransactionError::InstructionError(_, ix_err) => ix_err,
            _ => unreachable!(), // Mollusk only uses `InstructionError` variant.
        }
    }

    pub(crate) fn extract_txn_program_result(
        raw_result: &Result<(), TransactionError>,
    ) -> TransactionProgramResult {
        match raw_result {
            Ok(()) => TransactionProgramResult::Success,
            Err(TransactionError::InstructionError(idx, ix_err)) => {
                let index = *idx as usize;
                if let Ok(program_error) = ProgramError::try_from(ix_err.clone()) {
                    TransactionProgramResult::Failure(index, program_error)
                } else {
                    TransactionProgramResult::UnknownError(index, ix_err.clone())
                }
            }
            _ => unreachable!(), // Mollusk only uses `InstructionError` variant.
        }
    }
}
