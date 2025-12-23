//! Core result types for SVM program execution.

use {
    solana_account::Account, solana_instruction::error::InstructionError,
    solana_program_error::ProgramError, solana_pubkey::Pubkey,
    solana_transaction_error::TransactionError,
};
#[cfg(feature = "inner-instructions")]
use {solana_message::SanitizedMessage, solana_transaction_status_client_types::InnerInstruction};

/// The result code of the program's execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgramResult {
    /// The program executed successfully.
    Success,
    /// The program returned an error.
    Failure(ProgramError),
    /// Mollusk encountered an error while executing the program.
    UnknownError(InstructionError),
}

impl ProgramResult {
    /// Returns `true` if the program succeeded.
    pub const fn is_ok(&self) -> bool {
        matches!(self, ProgramResult::Success)
    }

    /// Returns `true` if the program returned an error.
    pub const fn is_err(&self) -> bool {
        !self.is_ok()
    }
}

impl From<Result<(), InstructionError>> for ProgramResult {
    fn from(result: Result<(), InstructionError>) -> Self {
        match result {
            Ok(()) => ProgramResult::Success,
            Err(err) => {
                if let Ok(program_error) = ProgramError::try_from(err.clone()) {
                    ProgramResult::Failure(program_error)
                } else {
                    ProgramResult::UnknownError(err)
                }
            }
        }
    }
}

/// The overall result of the instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstructionResult {
    /// The number of compute units consumed by the instruction.
    pub compute_units_consumed: u64,
    /// The time taken to execute the instruction.
    pub execution_time: u64,
    /// The result code of the program's execution.
    pub program_result: ProgramResult,
    /// The raw result of the program's execution.
    pub raw_result: Result<(), InstructionError>,
    /// The return data produced by the instruction, if any.
    pub return_data: Vec<u8>,
    /// The resulting accounts after executing the instruction.
    ///
    /// This includes all accounts provided to the processor, in the order
    /// they were provided. Any accounts that were modified will maintain
    /// their original position in this list, but with updated state.
    pub resulting_accounts: Vec<(Pubkey, Account)>,
    /// Inner instructions (CPIs) invoked during the instruction execution.
    ///
    /// Each entry represents a cross-program invocation made by the program,
    /// including the invoked instruction and the stack height at which it
    /// was called.
    #[cfg(feature = "inner-instructions")]
    pub inner_instructions: Vec<InnerInstruction>,
    /// The compiled message used to execute the instruction.
    ///
    /// This can be used to map account indices in inner instructions back to
    /// their corresponding pubkeys via `message.account_keys()`.
    ///
    /// This is `None` when the result is loaded from a fuzz fixture, since
    /// fixtures don't contain the compiled message.
    #[cfg(feature = "inner-instructions")]
    pub message: Option<SanitizedMessage>,
}

impl Default for InstructionResult {
    fn default() -> Self {
        Self {
            compute_units_consumed: 0,
            execution_time: 0,
            program_result: ProgramResult::Success,
            raw_result: Ok(()),
            return_data: vec![],
            resulting_accounts: vec![],
            #[cfg(feature = "inner-instructions")]
            inner_instructions: vec![],
            #[cfg(feature = "inner-instructions")]
            message: None,
        }
    }
}

impl InstructionResult {
    /// Get an account from the resulting accounts by its pubkey.
    pub fn get_account(&self, pubkey: &Pubkey) -> Option<&Account> {
        self.resulting_accounts
            .iter()
            .find(|(k, _)| k == pubkey)
            .map(|(_, a)| a)
    }

    pub fn absorb(&mut self, other: Self) {
        self.compute_units_consumed += other.compute_units_consumed;
        self.execution_time += other.execution_time;
        self.program_result = other.program_result;
        self.raw_result = other.raw_result;
        self.return_data = other.return_data;
        self.resulting_accounts = other.resulting_accounts;
        #[cfg(feature = "inner-instructions")]
        {
            self.inner_instructions = other.inner_instructions;
            self.message = other.message;
        }
    }
}

/// The result code of the last program's execution and its index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransactionProgramResult {
    /// The program executed successfully.
    Success,
    /// The program at this index returned an error.
    Failure(usize, ProgramError),
    /// Mollusk encountered an error while executing the program.
    UnknownError(usize, InstructionError),
}

/// The overall result of the transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransactionResult {
    /// The number of compute units consumed by the transaction.
    pub compute_units_consumed: u64,
    /// The time taken to execute the transaction.
    pub execution_time: u64,
    /// The result code of the last program's execution and its index.
    pub program_result: TransactionProgramResult,
    /// The raw result of the program's execution.
    pub raw_result: Result<(), TransactionError>,
    /// The return data produced by the transaction, if any.
    pub return_data: Vec<u8>,
    /// The resulting accounts after executing the transaction.
    ///
    /// This includes all accounts provided to the processor, in the order
    /// they were provided. Any accounts that were modified will maintain
    /// their original position in this list, but with updated state.
    pub resulting_accounts: Vec<(Pubkey, Account)>,
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
