//! Module for converting to and from Mollusk SVM fuzz fixtures and Mollusk
//! types. These conversions allow Mollusk to eject fuzzing fixtures from
//! tests, amongst other things.
//!
//! Only available when the `fuzz` feature is enabled.

use {
    crate::{
        result::{InstructionResult, ProgramResult},
        sysvar::Sysvars,
        Mollusk,
    },
    agave_feature_set::FeatureSet,
    mollusk_svm_fuzz_fixture::{
        context::Context as FuzzContext, effects::Effects as FuzzEffects,
        sysvars::Sysvars as FuzzSysvars, Fixture as FuzzFixture,
    },
    solana_account::Account,
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_instruction::{error::InstructionError, Instruction},
    solana_pubkey::Pubkey,
    solana_slot_hashes::SlotHashes,
    solana_sysvar::last_restart_slot::LastRestartSlot,
};

impl From<&Sysvars> for FuzzSysvars {
    fn from(input: &Sysvars) -> Self {
        let slot_hashes = SlotHashes::new(&input.slot_hashes);
        Self {
            clock: input.clock.clone(),
            epoch_rewards: input.epoch_rewards.clone(),
            epoch_schedule: input.epoch_schedule.clone(),
            rent: input.rent.clone(),
            slot_hashes,
            stake_history: input.stake_history.clone(),
        }
    }
}

impl From<&FuzzSysvars> for Sysvars {
    fn from(input: &FuzzSysvars) -> Self {
        let slot_hashes = SlotHashes::new(&input.slot_hashes);
        Self {
            clock: input.clock.clone(),
            epoch_rewards: input.epoch_rewards.clone(),
            epoch_schedule: input.epoch_schedule.clone(),
            last_restart_slot: LastRestartSlot::default(),
            rent: input.rent.clone(),
            slot_hashes,
            stake_history: input.stake_history.clone(),
        }
    }
}

impl From<&InstructionResult> for FuzzEffects {
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

impl From<&FuzzEffects> for InstructionResult {
    fn from(input: &FuzzEffects) -> Self {
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

pub struct ParsedFixtureContext {
    pub accounts: Vec<(Pubkey, Account)>,
    pub compute_budget: ComputeBudget,
    pub feature_set: FeatureSet,
    pub instruction: Instruction,
    pub sysvars: Sysvars,
}

fn build_fixture_context(
    accounts: &[(Pubkey, Account)],
    compute_budget: &ComputeBudget,
    feature_set: &FeatureSet,
    instruction: &Instruction,
    sysvars: &Sysvars,
) -> FuzzContext {
    let instruction_accounts = instruction.accounts.clone();
    let instruction_data = instruction.data.clone();
    let accounts = accounts.to_vec();

    FuzzContext {
        compute_budget: *compute_budget,
        feature_set: feature_set.clone(),
        sysvars: sysvars.into(),
        program_id: instruction.program_id,
        instruction_accounts,
        instruction_data,
        accounts,
    }
}

pub(crate) fn parse_fixture_context(context: &FuzzContext) -> ParsedFixtureContext {
    let FuzzContext {
        compute_budget,
        feature_set,
        sysvars,
        program_id,
        instruction_accounts,
        instruction_data,
        accounts,
    } = context;

    let instruction =
        Instruction::new_with_bytes(*program_id, instruction_data, instruction_accounts.clone());

    ParsedFixtureContext {
        accounts: accounts.clone(),
        compute_budget: *compute_budget,
        feature_set: feature_set.clone(),
        instruction,
        sysvars: sysvars.into(),
    }
}

pub fn build_fixture_from_mollusk_test(
    mollusk: &Mollusk,
    instruction: &Instruction,
    accounts: &[(Pubkey, Account)],
    result: &InstructionResult,
) -> FuzzFixture {
    let input = build_fixture_context(
        accounts,
        &mollusk.compute_budget,
        &mollusk.feature_set,
        instruction,
        &mollusk.sysvars,
    );
    // This should probably be built from the checks, but there's currently no
    // mechanism to enforce full check coverage on a result.
    let output = FuzzEffects::from(result);
    FuzzFixture { input, output }
}

pub fn load_fixture(
    fixture: &mollusk_svm_fuzz_fixture::Fixture,
) -> (ParsedFixtureContext, InstructionResult) {
    (
        parse_fixture_context(&fixture.input),
        InstructionResult::from(&fixture.output),
    )
}
