//! Re-execution of a historical transaction with Mollusk.

#![allow(deprecated)] // `RecentBlockhashes` is deprecated, but still a sysvar.

use {
    crate::{context::ReplayContext, error::ReplayError},
    agave_feature_set::{FeatureSet, FEATURE_NAMES},
    agave_reserved_account_keys::ReservedAccountKeys,
    mollusk_svm::{
        program::{loader_keys, ProgramCache},
        result::types::{TransactionProgramResult, TransactionResult},
        sysvar::Sysvars,
        Mollusk,
    },
    serde::de::DeserializeOwned,
    solana_account::Account,
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_compute_budget_instruction::instructions_processor::process_compute_budget_instructions,
    solana_loader_v3_interface::state::UpgradeableLoaderState,
    solana_message::SanitizedMessage,
    solana_program_runtime::execution_budget::SVMTransactionExecutionBudget,
    solana_pubkey::Pubkey,
    solana_svm_transaction::svm_message::{SVMMessage, SVMStaticMessage},
    solana_sysvar::{
        clock::Clock, epoch_rewards::EpochRewards, epoch_schedule::EpochSchedule,
        last_restart_slot::LastRestartSlot, recent_blockhashes::RecentBlockhashes, rent::Rent,
        slot_hashes::SlotHashes, stake_history::StakeHistory,
    },
    solana_sysvar_id::SysvarId,
    solana_transaction_error::TransactionError,
    std::collections::HashSet,
};

/// A program loaded by the transaction, with the ELF it held on-chain.
struct HistoricalProgram {
    program_id: Pubkey,
    loader_key: Pubkey,
    elf: Vec<u8>,
}

/// The parts of the execution environment derived from the feature set.
struct Environment {
    feature_set: FeatureSet,
    message: SanitizedMessage,
    compute_budget: ComputeBudget,
}

impl Environment {
    fn new(
        context: &ReplayContext,
        feature_set: FeatureSet,
        reserved_account_keys: &HashSet<Pubkey>,
    ) -> Result<Self, ReplayError> {
        let message = context.sanitize_message(reserved_account_keys)?;
        let compute_budget = compute_budget(&feature_set, &message)?;
        Ok(Self {
            feature_set,
            message,
            compute_budget,
        })
    }
}

/// Replays a historical transaction, either against its historical program
/// bytes or with one program swapped for a candidate build.
pub struct Replayer {
    context: ReplayContext,
    /// The accounts passed to Mollusk: the message's account keys first, in
    /// order, followed by any other context accounts. The fee is already
    /// deducted from the fee payer.
    accounts: Vec<(Pubkey, Account)>,
    programs: Vec<HistoricalProgram>,
    sysvars: Sysvars,
    /// The environment the transaction executed in on-chain.
    historical: Environment,
    /// The environment of candidate runs, if features were activated for
    /// them.
    candidate: Option<Environment>,
}

impl Replayer {
    /// Validate a replay context and prepare it for execution.
    pub fn new(context: ReplayContext) -> Result<Self, ReplayError> {
        let historical = Environment::new(
            &context,
            context.feature_set.clone(),
            &context.reserved_account_keys,
        )?;

        if SVMMessage::get_durable_nonce(&historical.message).is_some() {
            return Err(ReplayError::UnsupportedDurableNonce);
        }

        match &context.recorded_status {
            Ok(())
            | Err(TransactionError::InstructionError(..))
            | Err(TransactionError::InsufficientFundsForRent { .. }) => {}
            Err(err) => return Err(ReplayError::TransactionNotExecuted(err.clone())),
        }

        let builtins = ProgramCache::new(
            &historical.feature_set.runtime_features(),
            &historical.compute_budget,
            /* enable_register_tracing */ false,
        );
        let accounts = order_accounts(&context, &historical.message, &builtins)?;
        let programs = historical_programs(&historical.message, &accounts, &builtins)?;
        let sysvars = decode_sysvars(&accounts)?;

        Ok(Self {
            context,
            accounts,
            programs,
            sysvars,
            historical,
            candidate: None,
        })
    }

    /// Activate features for candidate runs, on top of those active at the
    /// transaction's slot. Replays against the historical program bytes,
    /// including the sanity check, always use the historical feature set.
    ///
    /// The candidate's feature set, reserved account keys, and compute budget
    /// are all derived from the activated features. One-time effects the
    /// runtime applies when a feature activates, such as migrating a builtin
    /// to a BPF program, are not emulated.
    pub fn activate_candidate_features(
        &mut self,
        feature_ids: &[Pubkey],
    ) -> Result<(), ReplayError> {
        if let Some(unknown) = feature_ids
            .iter()
            .find(|feature_id| !FEATURE_NAMES.contains_key(*feature_id))
        {
            return Err(ReplayError::UnknownFeature(*unknown));
        }

        let mut feature_set = self.candidate_environment().feature_set.clone();
        for feature_id in feature_ids {
            if !feature_set.is_active(feature_id) {
                feature_set.activate(feature_id, self.context.slot);
            }
        }

        let mut reserved_account_keys = ReservedAccountKeys::default();
        reserved_account_keys.update_active_set(&feature_set);
        reserved_account_keys
            .active
            .extend(self.context.reserved_account_keys.iter().copied());

        self.candidate = Some(Environment::new(
            &self.context,
            feature_set,
            &reserved_account_keys.active,
        )?);
        Ok(())
    }

    fn candidate_environment(&self) -> &Environment {
        self.candidate.as_ref().unwrap_or(&self.historical)
    }

    /// The context this replayer was built from.
    pub fn context(&self) -> &ReplayContext {
        &self.context
    }

    /// The IDs of all on-chain programs loaded by the transaction. Any of them
    /// can be swapped for a candidate build.
    pub fn program_ids(&self) -> impl Iterator<Item = &Pubkey> {
        self.programs.iter().map(|program| &program.program_id)
    }

    /// Replay the transaction against its historical program bytes.
    pub fn replay(&self) -> TransactionResult {
        self.mollusk(&self.historical)
            .process_sanitized_message(&self.historical.message, &self.accounts)
    }

    /// Replay the transaction with `program_id` swapped for the provided ELF,
    /// under any features activated with `activate_candidate_features`.
    ///
    /// Only the executable is swapped. The program's accounts, including its
    /// program data account, keep their historical contents.
    pub fn replay_with_program(
        &self,
        program_id: &Pubkey,
        elf: &[u8],
    ) -> Result<TransactionResult, ReplayError> {
        let program = self
            .programs
            .iter()
            .find(|program| program.program_id == *program_id)
            .ok_or(ReplayError::ProgramNotInTransaction(*program_id))?;

        let environment = self.candidate_environment();
        let mut mollusk = self.mollusk(environment);
        mollusk
            .program_cache
            .add_program(program_id, &program.loader_key, elf);

        Ok(mollusk.process_sanitized_message(&environment.message, &self.accounts))
    }

    /// Replay the transaction against its historical program bytes, requiring
    /// the recorded execution result to be reproduced, then replay it with
    /// `program_id` swapped for the provided ELF, under any features activated
    /// with `activate_candidate_features`.
    pub fn verify(&self, program_id: &Pubkey, elf: &[u8]) -> Result<ReplayReport, ReplayError> {
        let baseline = self.replay();
        let replayed = execution_result(&baseline);
        if replayed != self.context.recorded_status {
            return Err(ReplayError::SanityCheckFailed {
                recorded: self.context.recorded_status.clone(),
                replayed,
            });
        }

        let candidate = self.replay_with_program(program_id, elf)?;

        Ok(ReplayReport {
            recorded: self.context.recorded_status.clone(),
            baseline,
            candidate,
        })
    }

    fn mollusk(&self, environment: &Environment) -> Mollusk {
        let mut mollusk = Mollusk::default();
        mollusk.compute_budget = environment.compute_budget;
        mollusk.epoch_stake = self.context.epoch_stake.clone();
        mollusk.feature_set = environment.feature_set.runtime_features();
        mollusk.sysvars = clone_sysvars(&self.sysvars);
        // The program cache captures the runtime environment, so it must be
        // rebuilt for the historical feature set and compute budget.
        mollusk.program_cache = ProgramCache::new(
            &mollusk.feature_set,
            &mollusk.compute_budget,
            /* enable_register_tracing */ false,
        );
        // Report rent violations as results rather than panicking.
        mollusk.config.panic = false;

        for program in &self.programs {
            mollusk.program_cache.add_program(
                &program.program_id,
                &program.loader_key,
                &program.elf,
            );
        }

        mollusk
    }
}

/// The results of replaying a transaction against its historical program
/// bytes (the baseline) and a candidate build.
#[derive(Debug)]
pub struct ReplayReport {
    /// The execution result recorded in the ledger.
    pub recorded: Result<(), TransactionError>,
    /// The replay against the historical program bytes.
    pub baseline: TransactionResult,
    /// The replay against the candidate program bytes.
    pub candidate: TransactionResult,
}

impl ReplayReport {
    /// Whether the candidate reproduced the recorded execution result.
    pub fn candidate_matches(&self) -> bool {
        execution_result(&self.candidate) == self.recorded
    }
}

/// The execution result of a Mollusk transaction, in the form the runtime
/// records it in the ledger.
pub fn execution_result(result: &TransactionResult) -> Result<(), TransactionError> {
    match &result.program_result {
        TransactionProgramResult::TransactionError(err) => Err(err.clone()),
        _ => result.raw_result.clone(),
    }
}

/// Order the context accounts to match the message's account keys, so
/// transaction-level account indices (ie. in rent errors) line up with the
/// runtime's, then deduct the fee from the fee payer.
///
/// Account keys missing from the context are treated as nonexistent
/// accounts, except for builtin programs, which are stubbed as the runtime
/// provides them.
///
/// The Instructions sysvar is omitted entirely: it is a virtual account the
/// runtime builds per transaction, never stored on-chain, so it is absent from
/// the context. Mollusk rebuilds it from the message, but only if it is not
/// otherwise provided. Stubbing it here would shadow that with a default
/// (system-owned) account, which the runtime rejects with `InvalidAccountOwner`
/// at the start of every instruction.
fn order_accounts(
    context: &ReplayContext,
    message: &SanitizedMessage,
    builtins: &ProgramCache,
) -> Result<Vec<(Pubkey, Account)>, ReplayError> {
    let account_keys = message.account_keys();
    let mut accounts = Vec::with_capacity(context.accounts.len().max(account_keys.len()));
    for (index, key) in account_keys.iter().enumerate() {
        if *key == solana_sdk_ids::sysvar::instructions::id() {
            continue;
        }
        let account = match context.accounts.iter().find(|(k, _)| k == key) {
            Some((_, account)) => account.clone(),
            None if is_builtin(key, builtins) => Account {
                owner: loader_keys::NATIVE_LOADER,
                executable: true,
                ..Account::default()
            },
            None if message.is_invoked(index) => {
                return Err(ReplayError::InvalidAccount {
                    address: *key,
                    reason: "missing program account".to_owned(),
                });
            }
            None => Account::default(),
        };
        accounts.push((*key, account));
    }
    accounts.extend(
        context
            .accounts
            .iter()
            .filter(|(key, _)| !account_keys.iter().any(|k| k == key))
            .cloned(),
    );

    // The fee payer is always the first account key.
    if let Some((_, fee_payer)) = accounts.first_mut() {
        fee_payer.lamports = fee_payer.lamports.checked_sub(context.fee).ok_or(
            ReplayError::TransactionNotExecuted(TransactionError::InsufficientFundsForFee),
        )?;
    }

    Ok(accounts)
}

/// Derive the transaction's compute budget from its compute budget
/// instructions, as the runtime does.
fn compute_budget(
    feature_set: &FeatureSet,
    message: &SanitizedMessage,
) -> Result<ComputeBudget, ReplayError> {
    let limits = process_compute_budget_instructions(
        SVMStaticMessage::program_instructions_iter(message),
        feature_set,
    )
    .map_err(|err| ReplayError::InvalidMessage(err.to_string()))?;

    let simd_0268_active = feature_set.runtime_features().raise_cpi_nesting_limit_to_8;
    let budget = SVMTransactionExecutionBudget {
        compute_unit_limit: u64::from(limits.compute_unit_limit),
        heap_size: limits.updated_heap_bytes,
        ..SVMTransactionExecutionBudget::new_with_defaults(simd_0268_active)
    };
    let cost = ComputeBudget::new_with_defaults(simd_0268_active).to_cost();

    Ok(ComputeBudget::from_budget_and_cost(&budget, &cost))
}

/// Collect the ELF of every on-chain program among the message's account
/// keys, and ensure every invoked builtin is provided by Mollusk.
///
/// Cross-program invocation targets must be account keys of the message, so
/// this covers every program the transaction can execute.
fn historical_programs(
    message: &SanitizedMessage,
    accounts: &[(Pubkey, Account)],
    builtins: &ProgramCache,
) -> Result<Vec<HistoricalProgram>, ReplayError> {
    let mut programs = Vec::new();
    for (index, (program_id, account)) in accounts
        .iter()
        .take(message.account_keys().len())
        .enumerate()
    {
        match account.owner {
            loader_keys::NATIVE_LOADER => {
                // Unsupported builtins are only rejected when invoked at the
                // top level. A cross-program invocation of one surfaces as a
                // sanity check failure.
                if !is_builtin(program_id, builtins) && message.is_invoked(index) {
                    return Err(ReplayError::UnsupportedBuiltin(*program_id));
                }
            }
            loader_keys::LOADER_V1 | loader_keys::LOADER_V2 if account.executable => {
                programs.push(HistoricalProgram {
                    program_id: *program_id,
                    loader_key: account.owner,
                    elf: account.data.clone(),
                });
            }
            loader_keys::LOADER_V3 => {
                if let Some(elf) = loader_v3_elf(program_id, account, accounts)? {
                    programs.push(HistoricalProgram {
                        program_id: *program_id,
                        loader_key: account.owner,
                        elf,
                    });
                }
            }
            loader if loader == solana_sdk_ids::loader_v4::id() && account.executable => {
                return Err(ReplayError::UnsupportedLoader {
                    program_id: *program_id,
                    loader,
                });
            }
            _ => {}
        }
    }

    Ok(programs)
}

/// Whether Mollusk provides the native program, either as a builtin or a
/// precompile.
fn is_builtin(program_id: &Pubkey, builtins: &ProgramCache) -> bool {
    *program_id == solana_sdk_ids::ed25519_program::id()
        || *program_id == solana_sdk_ids::secp256k1_program::id()
        || *program_id == solana_sdk_ids::secp256r1_program::id()
        || builtins.load_program(program_id).is_some()
}

/// Extract the ELF of a loader-v3 program from its program data account.
///
/// Returns `None` if the account is not a program (ie. a buffer or program
/// data account), or if the program has been closed.
fn loader_v3_elf(
    program_id: &Pubkey,
    account: &Account,
    accounts: &[(Pubkey, Account)],
) -> Result<Option<Vec<u8>>, ReplayError> {
    let Ok(UpgradeableLoaderState::Program {
        programdata_address,
    }) = bincode::deserialize(&account.data)
    else {
        return Ok(None);
    };

    let (_, programdata) = accounts
        .iter()
        .find(|(key, _)| *key == programdata_address)
        .ok_or_else(|| ReplayError::InvalidAccount {
            address: *program_id,
            reason: format!("missing program data account {programdata_address}"),
        })?;

    let Ok(UpgradeableLoaderState::ProgramData { .. }) = bincode::deserialize(&programdata.data)
    else {
        return Ok(None);
    };

    Ok(programdata
        .data
        .get(UpgradeableLoaderState::size_of_programdata_metadata()..)
        .filter(|elf| !elf.is_empty())
        .map(<[u8]>::to_vec))
}

fn decode_sysvars(accounts: &[(Pubkey, Account)]) -> Result<Sysvars, ReplayError> {
    fn decode<T: DeserializeOwned + SysvarId>(
        accounts: &[(Pubkey, Account)],
        sysvar: &mut T,
    ) -> Result<(), ReplayError> {
        let Some((address, account)) = accounts.iter().find(|(key, _)| *key == T::id()) else {
            return Ok(());
        };
        *sysvar =
            bincode::deserialize(&account.data).map_err(|err| ReplayError::InvalidAccount {
                address: *address,
                reason: err.to_string(),
            })?;
        Ok(())
    }

    let mut sysvars = Sysvars::default();
    decode::<Clock>(accounts, &mut sysvars.clock)?;
    decode::<EpochRewards>(accounts, &mut sysvars.epoch_rewards)?;
    decode::<EpochSchedule>(accounts, &mut sysvars.epoch_schedule)?;
    decode::<LastRestartSlot>(accounts, &mut sysvars.last_restart_slot)?;
    decode::<Rent>(accounts, &mut sysvars.rent)?;
    decode::<SlotHashes>(accounts, &mut sysvars.slot_hashes)?;
    decode::<StakeHistory>(accounts, &mut sysvars.stake_history)?;
    decode::<RecentBlockhashes>(accounts, &mut sysvars.recent_blockhashes)?;
    Ok(sysvars)
}

fn clone_sysvars(sysvars: &Sysvars) -> Sysvars {
    Sysvars {
        clock: sysvars.clock.clone(),
        epoch_rewards: sysvars.epoch_rewards.clone(),
        epoch_schedule: sysvars.epoch_schedule.clone(),
        last_restart_slot: sysvars.last_restart_slot.clone(),
        rent: sysvars.rent.clone(),
        slot_hashes: SlotHashes::new(&sysvars.slot_hashes),
        stake_history: sysvars.stake_history.clone(),
        recent_blockhashes: sysvars.recent_blockhashes.clone(),
    }
}
