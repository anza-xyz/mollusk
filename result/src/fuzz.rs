//! Fuzz fixture conversions for instruction results.

use crate::types::{InstructionResult, ProgramResult};

impl From<&InstructionResult> for mollusk_svm_fuzz_fixture::effects::Effects {
    fn from(input: &InstructionResult) -> Self {
        let compute_units_consumed = input.compute_units_consumed;
        let execution_time = input.execution_time;
        let return_data = input.return_data.clone();

        let program_result = match &input.program_result {
            ProgramResult::Success => 0,
            ProgramResult::Failure(e) => u64::from(e.clone()),
            ProgramResult::UnknownError(_) => u64::MAX, //TODO
        };

        let resulting_accounts = input.resulting_accounts.clone();

        Self {
            compute_units_consumed,
            execution_time,
            program_result,
            return_data,
            resulting_accounts,
        }
    }
}

impl From<&mollusk_svm_fuzz_fixture::effects::Effects> for InstructionResult {
    fn from(input: &mollusk_svm_fuzz_fixture::effects::Effects) -> Self {
        use solana_instruction::error::InstructionError;

        let compute_units_consumed = input.compute_units_consumed;
        let execution_time = input.execution_time;
        let return_data = input.return_data.clone();

        let raw_result = if input.program_result == 0 {
            Ok(())
        } else {
            Err(InstructionError::from(input.program_result))
        };

        let program_result = raw_result.clone().into();

        let resulting_accounts = input.resulting_accounts.clone();

        Self {
            compute_units_consumed,
            execution_time,
            program_result,
            raw_result,
            return_data,
            resulting_accounts,
        }
    }
}
