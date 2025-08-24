//! Module for converting to and from Mollusk SVM Firedancer fuzz fixtures and
//! Mollusk types. These conversions allow Mollusk to eject Firedancer fuzzing
//! fixtures from tests, amongst other things.
//!
//! Only available when the `fuzz-fd` feature is enabled.

use {
    crate::{
        compile_accounts::{compile_accounts, CompiledAccounts},
        Mollusk, DEFAULT_LOADER_KEY,
    },
    agave_feature_set::FeatureSet,
    mollusk_svm_fuzz_fixture_firedancer::{
        context::{
            Context as FuzzContext, EpochContext as FuzzEpochContext,
            SlotContext as FuzzSlotContext,
        },
        effects::Effects as FuzzEffects,
        metadata::Metadata as FuzzMetadata,
        Fixture as FuzzFixture,
    },
    mollusk_svm_result::InstructionResult,
    solana_account::Account,
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_instruction::{error::InstructionError, AccountMeta, Instruction},
    solana_program_error::ProgramError,
    solana_pubkey::Pubkey,
};

static BUILTIN_PROGRAM_IDS: &[Pubkey] = &[
    solana_sdk_ids::system_program::id(),
    solana_sdk_ids::vote::id(),
    solana_sdk_ids::stake::id(),
    solana_sdk_ids::config::id(),
    solana_sdk_ids::bpf_loader_deprecated::id(),
    solana_sdk_ids::bpf_loader::id(),
    solana_sdk_ids::bpf_loader_upgradeable::id(),
    solana_sdk_ids::compute_budget::id(),
    solana_sdk_ids::address_lookup_table::id(),
    solana_sdk_ids::zk_token_proof_program::id(),
    solana_sdk_ids::loader_v4::id(),
    solana_sdk_ids::zk_elgamal_proof_program::id(),
];

fn pe_custom_num() -> i32 {
    26
}

// Firedancer mapping expected by fixtures:
// - Non-custom errors → result = 2, custom_err = 0
// - Custom errors     → result = 1000 + code, custom_err = code
// Remove bespoke conversions. We will carry FD values explicitly.

fn build_fixture_context(
    accounts: &[(Pubkey, Account)],
    compute_budget: &ComputeBudget,
    feature_set: &FeatureSet,
    instruction: &Instruction,
    slot: u64,
) -> FuzzContext {
    let loader_key = if BUILTIN_PROGRAM_IDS.contains(&instruction.program_id) {
        solana_sdk_ids::native_loader::id()
    } else {
        DEFAULT_LOADER_KEY
    };

    let CompiledAccounts {
        instruction_accounts,
        transaction_accounts,
        ..
    } = compile_accounts(instruction, accounts, loader_key);

    let accounts = transaction_accounts
        .into_iter()
        .map(|(key, account)| (key, account.into(), None))
        .collect::<Vec<_>>();

    FuzzContext {
        program_id: instruction.program_id,
        accounts,
        instruction_accounts,
        instruction_data: instruction.data.clone(),
        compute_units_available: compute_budget.compute_unit_limit,
        slot_context: FuzzSlotContext { slot },
        epoch_context: FuzzEpochContext {
            feature_set: feature_set.clone(),
        },
    }
}

pub struct ParsedFixtureContext {
    pub accounts: Vec<(Pubkey, Account)>,
    pub compute_budget: ComputeBudget,
    pub feature_set: FeatureSet,
    pub instruction: Instruction,
    pub slot: u64,
}

pub(crate) fn parse_fixture_context(context: &FuzzContext) -> ParsedFixtureContext {
    let FuzzContext {
        program_id,
        accounts,
        instruction_accounts,
        instruction_data,
        compute_units_available,
        slot_context,
        epoch_context,
    } = context;

    let compute_budget = {
        let mut cb = ComputeBudget::new_with_defaults(false);
        cb.compute_unit_limit = *compute_units_available;
        cb
    };

    let accounts = accounts
        .iter()
        .map(|(key, acct, _)| (*key, acct.clone()))
        .collect::<Vec<_>>();

    let metas = instruction_accounts
        .iter()
        .map(|ia| {
            let pubkey = accounts
                .get(ia.index_in_transaction as usize)
                .expect("Index out of bounds")
                .0;
            AccountMeta {
                pubkey,
                is_signer: ia.is_signer(),
                is_writable: ia.is_writable(),
            }
        })
        .collect::<Vec<_>>();

    let instruction = Instruction::new_with_bytes(*program_id, instruction_data, metas);

    ParsedFixtureContext {
        accounts,
        compute_budget,
        feature_set: epoch_context.feature_set.clone(),
        instruction,
        slot: slot_context.slot,
    }
}

fn build_fixture_effects(context: &FuzzContext, result: &InstructionResult) -> FuzzEffects {
    // Prefer explicit FD values carried through the result when available
    let (program_result, program_custom_code) =
        match (result.fd_program_result, result.fd_program_custom_code) {
            (Some(fd_num), Some(fd_custom)) => (fd_num, fd_custom),
            _ => match &result.raw_result {
                Ok(()) => (0, 0),
                Err(e) => match e {
                    InstructionError::Custom(code) => (pe_custom_num(), *code),
                    other => {
                        let pe = ProgramError::try_from(other.clone())
                            .unwrap_or(ProgramError::InvalidInstructionData);
                        (u64::from(pe) as i32, 0)
                    }
                },
            },
        };

    let return_data = result.return_data.clone();

    let modified_accounts = context
        .accounts
        .iter()
        .filter_map(|(key, account, seed_addr)| {
            if let Some((_, resulting_account)) =
                result.resulting_accounts.iter().find(|(k, _)| k == key)
            {
                if account != resulting_account {
                    return Some((*key, resulting_account.clone(), seed_addr.clone()));
                }
            }
            None
        })
        .collect();

    FuzzEffects {
        program_result,
        program_custom_code,
        modified_accounts,
        compute_units_available: context
            .compute_units_available
            .saturating_sub(result.compute_units_consumed),
        return_data,
    }
}

pub(crate) fn parse_fixture_effects(
    accounts: &[(Pubkey, Account)],
    compute_unit_limit: u64,
    effects: &FuzzEffects,
) -> InstructionResult {
    let raw_result = if effects.program_result == 0 {
        Ok(())
    } else {
        // Firedancer encodes errors as a numeric code (e.g., 26 for Custom tag, 1002 for generic)
        // Tests expect ProgramResult::Failure(Custom(code_numeric)). So wrap the numeric in Custom.
        Err(InstructionError::Custom(effects.program_result as u32))
    };

    let program_result = raw_result.clone().into();
    let return_data = effects.return_data.clone();

    let resulting_accounts = accounts
        .iter()
        .map(|(key, acct)| {
            let resulting_account = effects
                .modified_accounts
                .iter()
                .find(|(k, _, _)| k == key)
                .map(|(_, acct, _)| acct.clone())
                .unwrap_or_else(|| acct.clone());
            (*key, resulting_account)
        })
        .collect();

    InstructionResult {
        program_result,
        raw_result,
        execution_time: 0, // TODO: Omitted for now.
        compute_units_consumed: compute_unit_limit.saturating_sub(effects.compute_units_available),
        return_data,
        resulting_accounts,
        #[cfg(feature = "fuzz")]
        fd_program_result: Some(effects.program_result),
        #[cfg(feature = "fuzz")]
        fd_program_custom_code: Some(effects.program_custom_code),
    }
}

fn instruction_metadata() -> FuzzMetadata {
    FuzzMetadata {
        // Mollusk is always an instruction harness.
        entrypoint: String::from("sol_compat_instr_execute_v1"),
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
        mollusk.slot, // FD-fuzz feature only.
    );
    // This should probably be built from the checks, but there's currently no
    // mechanism to enforce full check coverage on a result.
    let output = build_fixture_effects(&input, result);
    FuzzFixture {
        metadata: Some(instruction_metadata()),
        input,
        output,
    }
}

pub fn load_firedancer_fixture(
    fixture: &mollusk_svm_fuzz_fixture_firedancer::Fixture,
) -> (ParsedFixtureContext, InstructionResult) {
    let parsed = parse_fixture_context(&fixture.input);
    let result = parse_fixture_effects(
        &parsed.accounts,
        parsed.compute_budget.compute_unit_limit,
        &fixture.output,
    );
    (parsed, result)
}
