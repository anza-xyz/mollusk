//! Loading replay contexts from an Agave ledger directory.
//!
//! An Agave blockstore records transactions and their status meta, but not
//! account state. To recover the state a transaction observed, the bank is
//! loaded from the highest snapshot archive at or below the transaction's
//! parent slot, the ledger is replayed up to the parent slot, and the
//! transactions preceding the target in its own block are executed.

use {
    crate::{context::ReplayContext, error::ReplayError, progress},
    agave_snapshots::{
        paths::{
            full_snapshot_archives_iter, incremental_snapshot_archives_iter, BANK_SNAPSHOTS_DIR,
        },
        snapshot_archive_info::SnapshotArchiveInfoGetter,
        snapshot_config::SnapshotConfig,
    },
    solana_account::Account,
    solana_accounts_db::utils::create_accounts_run_and_snapshot_dirs,
    solana_clock::Slot,
    solana_genesis_config::GenesisConfig,
    solana_genesis_utils::{open_genesis_config, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE},
    solana_leader_schedule::SlotLeader,
    solana_ledger::{
        blockstore::Blockstore,
        blockstore_options::{AccessType, BlockstoreOptions},
        blockstore_processor::{process_blockstore_from_root, ProcessOptions},
        leader_schedule_cache::LeaderScheduleCache,
    },
    solana_loader_v3_interface::state::UpgradeableLoaderState,
    solana_pubkey::Pubkey,
    solana_runtime::{bank::Bank, bank_forks::BankForks, snapshot_bank_utils},
    solana_sdk_ids::sysvar,
    solana_signature::Signature,
    solana_transaction_status::VersionedTransactionWithStatusMeta,
    solana_unified_scheduler_pool::DefaultSchedulerPool,
    std::{
        collections::HashSet,
        fmt::Display,
        path::{Path, PathBuf},
        sync::{atomic::AtomicBool, Arc, RwLock},
    },
};

/// The directory, relative to the ledger, that accounts are unpacked into by
/// default.
const DEFAULT_WORK_DIR: &str = "mollusk-replay";

/// Sysvar accounts captured alongside the transaction's own accounts, so
/// programs reading sysvars through syscalls observe historical values.
const SYSVARS: &[Pubkey] = &[
    sysvar::clock::ID,
    sysvar::epoch_rewards::ID,
    sysvar::epoch_schedule::ID,
    sysvar::fees::ID,
    sysvar::last_restart_slot::ID,
    sysvar::recent_blockhashes::ID,
    sysvar::rent::ID,
    sysvar::slot_hashes::ID,
    sysvar::slot_history::ID,
    sysvar::stake_history::ID,
];

/// Where to find the snapshot to replay from, and where to unpack it.
#[derive(Clone, Debug, Default)]
pub struct LedgerConfig {
    /// The genesis to load, either a `genesis.bin` file or a directory
    /// containing one (or a `genesis.tar.bz2` archive). Defaults to the ledger
    /// directory.
    pub genesis: Option<PathBuf>,
    /// The directory containing full and incremental snapshot archives.
    /// Defaults to the ledger directory.
    pub snapshot_archives_dir: Option<PathBuf>,
    /// The directory accounts are unpacked into. Defaults to
    /// `<ledger>/mollusk-replay`.
    pub work_dir: Option<PathBuf>,
}

impl ReplayContext {
    /// Load the pre-execution state of a rooted transaction from an Agave
    /// ledger directory.
    ///
    /// The ledger must contain `genesis.bin`, a snapshot archive at or below
    /// the transaction's parent slot, and every block from that snapshot up to
    /// the transaction's own. The blockstore is opened read-only.
    pub fn from_ledger(
        ledger_path: &Path,
        signature: &Signature,
        config: &LedgerConfig,
    ) -> Result<Self, ReplayError> {
        progress::header("Building replay context from ledger");

        let progress = progress::step("opening blockstore (read-only)");
        let blockstore = Blockstore::open_with_options(
            ledger_path,
            BlockstoreOptions {
                access_type: AccessType::ReadOnly,
                ..BlockstoreOptions::default()
            },
        )
        .map_err(ledger_error)?;
        progress.done();

        // 1) Locate the transaction within its block.
        let progress = progress::step("locating transaction");
        let (slot, _) = blockstore
            .get_rooted_transaction_status(*signature)
            .map_err(ledger_error)?
            .ok_or_else(|| {
                ReplayError::Ledger(format!(
                    "transaction {signature} not found in a rooted slot"
                ))
            })?;
        let block = blockstore
            .get_complete_block(slot, /* require_previous_blockhash */ false)
            .map_err(ledger_error)?;
        let position = block
            .transactions
            .iter()
            .position(|tx| tx.transaction.signatures.first() == Some(signature))
            .ok_or_else(|| {
                ReplayError::Ledger(format!("transaction {signature} not found in slot {slot}"))
            })?;
        progress.done_with(&format!(
            "slot {slot} · transaction {} of {}",
            position + 1,
            block.transactions.len(),
        ));

        // 2) Load the bank of the parent slot.
        let bank_forks = load_bank_forks(ledger_path, &blockstore, block.parent_slot, config)?;
        let parent = bank_forks
            .read()
            .map_err(ledger_error)?
            .get(block.parent_slot)
            .ok_or_else(|| {
                ReplayError::Ledger(format!("slot {} was not replayed", block.parent_slot))
            })?;

        // 3) Replay the transactions preceding the target in its block. The
        // bank must be tracked by `bank_forks`, which its program cache
        // consults as its fork graph. It is created before taking the write
        // lock, since creating it reads the fork graph.
        let leader = LeaderScheduleCache::new_from_bank(&parent)
            .slot_leader_at(slot, Some(&parent))
            .ok_or_else(|| ReplayError::Ledger(format!("no leader for slot {slot}")))?;
        let bank = Bank::new_from_parent(parent, leader, slot);
        let bank = bank_forks
            .write()
            .map_err(ledger_error)?
            .insert(bank)
            .clone_without_scheduler();
        if position > 0 {
            let progress = progress::step(&format!(
                "replaying {position} preceding transaction(s) in the block"
            ));
            for preceding in &block.transactions[..position] {
                replay_preceding_transaction(&bank, preceding)?;
            }
            progress.done();
        }

        // 4) Capture the state visible to the target transaction.
        let progress = progress::step("capturing account state");
        let context = capture_context(&bank, &block.transactions[position]);
        progress.done_with(&format!("{} accounts", context.accounts.len()));
        Ok(context)
    }
}

fn ledger_error(err: impl Display) -> ReplayError {
    ReplayError::Ledger(err.to_string())
}

/// Load the genesis config.
///
/// `config.genesis` may point at a directory holding the genesis (a
/// `genesis.bin` file or a `genesis.tar.bz2` archive), or directly at either
/// file, under any name. When it is unset, the genesis is loaded from the ledger
/// directory.
fn load_genesis(ledger_path: &Path, config: &LedgerConfig) -> Result<GenesisConfig, ReplayError> {
    match config.genesis.as_deref() {
        None => open_genesis_config(ledger_path, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE)
            .map_err(ledger_error),
        Some(dir) if dir.is_dir() => {
            open_genesis_config(dir, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE).map_err(ledger_error)
        }
        Some(file) => load_genesis_file(file, ledger_path, config),
    }
}

/// Load the genesis from a file, which may be a raw `genesis.bin` or a
/// `genesis.tar.bz2` archive, regardless of its name.
fn load_genesis_file(
    file: &Path,
    ledger_path: &Path,
    config: &LedgerConfig,
) -> Result<GenesisConfig, ReplayError> {
    let bytes = std::fs::read(file).map_err(ledger_error)?;

    // A raw `genesis.bin` is bincode. An archive is bzip2-compressed, which
    // `open_genesis_config` unpacks, but only from a directory and only when
    // named `genesis.tar.bz2`, so stage it under that name first.
    if !bytes.starts_with(b"BZh") {
        return bincode::deserialize(&bytes).map_err(ledger_error);
    }

    let work_dir = config
        .work_dir
        .clone()
        .unwrap_or_else(|| ledger_path.join(DEFAULT_WORK_DIR));
    let stage = work_dir.join("genesis");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).map_err(ledger_error)?;
    std::fs::write(stage.join("genesis.tar.bz2"), &bytes).map_err(ledger_error)?;
    open_genesis_config(&stage, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE).map_err(ledger_error)
}

/// Load the bank at `slot` from the highest snapshot at or below it, replaying
/// the ledger from the snapshot if needed.
fn load_bank_forks(
    ledger_path: &Path,
    blockstore: &Blockstore,
    slot: Slot,
    config: &LedgerConfig,
) -> Result<Arc<RwLock<BankForks>>, ReplayError> {
    let genesis_config = load_genesis(ledger_path, config)?;

    let archives_dir = config
        .snapshot_archives_dir
        .clone()
        .unwrap_or_else(|| ledger_path.to_path_buf());
    let work_dir = config
        .work_dir
        .clone()
        .unwrap_or_else(|| ledger_path.join(DEFAULT_WORK_DIR));

    // Snapshots above `slot` already include the effects of the transaction,
    // so they can't be used.
    let full_snapshot_archive = full_snapshot_archives_iter(&archives_dir)
        .filter(|archive| archive.slot() <= slot)
        .max()
        .ok_or_else(|| {
            ReplayError::Ledger(format!(
                "no full snapshot archive at or below slot {slot} in {}",
                archives_dir.display()
            ))
        })?;
    let incremental_snapshot_archive = incremental_snapshot_archives_iter(&archives_dir)
        .filter(|archive| {
            archive.base_slot() == full_snapshot_archive.slot() && archive.slot() <= slot
        })
        .max();

    let loading = progress::step(&format!(
        "loading bank from snapshot @ slot {}{}",
        full_snapshot_archive.slot(),
        match &incremental_snapshot_archive {
            Some(archive) => format!(" (+ incremental @ {})", archive.slot()),
            None => String::new(),
        },
    ));

    let (account_run_path, _) =
        create_accounts_run_and_snapshot_dirs(work_dir.join("accounts")).map_err(ledger_error)?;
    let snapshot_config = SnapshotConfig {
        full_snapshot_archives_dir: archives_dir.clone(),
        incremental_snapshot_archives_dir: archives_dir,
        bank_snapshots_dir: work_dir.join(BANK_SNAPSHOTS_DIR),
        ..SnapshotConfig::new_load_only()
    };
    std::fs::create_dir_all(&snapshot_config.bank_snapshots_dir).map_err(ledger_error)?;

    let process_options = ProcessOptions {
        halt_at_slot: Some(slot),
        ..ProcessOptions::default()
    };

    let bank = snapshot_bank_utils::bank_from_snapshot_archives(
        &[account_run_path],
        &full_snapshot_archive,
        incremental_snapshot_archive.as_ref(),
        &snapshot_config,
        &genesis_config,
        &process_options.runtime_config,
        /* debug_keys */ None,
        /* leader_for_tests */ None::<SlotLeader>,
        /* limit_load_slot_count_from_snapshot */ None,
        /* accounts_db_force_initial_clean */ false,
        /* verify_index */ false,
        process_options.accounts_db_config.clone(),
        /* accounts_update_notifier */ None,
        Arc::new(AtomicBool::new(false)),
    )
    .map_err(ledger_error)?;
    loading.done_with(&format!("loaded at slot {}", bank.slot()));

    if bank.slot() == slot {
        return Ok(BankForks::new_rw_arc(bank));
    }

    let replaying = progress::step(&format!(
        "replaying {} slot(s) to slot {slot}",
        slot.saturating_sub(bank.slot()),
    ));

    let shred_version = solana_shred_version::compute_shred_version(
        &genesis_config.hash(),
        Some(&bank.hard_forks()),
    );
    let leader_schedule_cache = LeaderScheduleCache::new_from_bank(&bank);

    let bank_forks = BankForks::new_rw_arc(bank);
    bank_forks
        .write()
        .map_err(ledger_error)?
        // Replay requires a scheduler pool, with every option left at its
        // default.
        .install_scheduler_pool(DefaultSchedulerPool::new(
            None, None, None, None, None, None,
        ));

    process_blockstore_from_root(
        blockstore,
        &bank_forks,
        shred_version,
        &leader_schedule_cache,
        &process_options,
        /* transaction_status_sender */ None,
        /* entry_notification_sender */ None,
        /* snapshot_controller */ None,
    )
    .map_err(ledger_error)?;
    replaying.done();

    Ok(bank_forks)
}

/// Execute and commit a transaction preceding the target in its block,
/// requiring it to reproduce its recorded result.
fn replay_preceding_transaction(
    bank: &Bank,
    transaction: &VersionedTransactionWithStatusMeta,
) -> Result<(), ReplayError> {
    let signature = transaction
        .transaction
        .signatures
        .first()
        .copied()
        .unwrap_or_default();

    let status = bank
        .try_process_entry_transactions(vec![transaction.transaction.clone()])
        .map_err(|err| {
            ReplayError::Ledger(format!("preceding transaction {signature} rejected: {err}"))
        })?
        .pop()
        .ok_or_else(|| {
            ReplayError::Ledger(format!(
                "preceding transaction {signature} was not processed"
            ))
        })?;

    if status != transaction.meta.status {
        return Err(ReplayError::Ledger(format!(
            "preceding transaction {signature} replayed with {status:?}, but recorded {:?}",
            transaction.meta.status,
        )));
    }

    Ok(())
}

/// Capture every account visible to the transaction, along with the
/// environment it executed in.
fn capture_context(bank: &Bank, transaction: &VersionedTransactionWithStatusMeta) -> ReplayContext {
    let message = transaction.transaction.message.clone();
    let loaded_addresses = transaction.meta.loaded_addresses.clone();

    let mut seen = HashSet::new();
    let mut accounts: Vec<(Pubkey, Account)> = message
        .static_account_keys()
        .iter()
        .chain(&loaded_addresses.writable)
        .chain(&loaded_addresses.readonly)
        .chain(SYSVARS)
        .filter(|address| seen.insert(**address))
        .filter_map(|address| {
            bank.get_account(address)
                .map(|account| (*address, Account::from(account)))
        })
        .collect();

    let programdata_addresses: Vec<Pubkey> = accounts
        .iter()
        .filter(|(_, account)| account.owner == solana_sdk_ids::bpf_loader_upgradeable::id())
        .filter_map(|(_, account)| match bincode::deserialize(&account.data) {
            Ok(UpgradeableLoaderState::Program {
                programdata_address,
            }) => Some(programdata_address),
            _ => None,
        })
        .filter(|address| seen.insert(*address))
        .collect();
    accounts.extend(programdata_addresses.into_iter().filter_map(|address| {
        bank.get_account(&address)
            .map(|account| (address, Account::from(account)))
    }));

    let epoch_stake = bank
        .epoch_vote_accounts(bank.epoch())
        .map(|vote_accounts| {
            vote_accounts
                .iter()
                .map(|(address, (stake, _))| (*address, *stake))
                .collect()
        })
        .unwrap_or_default();

    ReplayContext {
        slot: bank.slot(),
        message,
        loaded_addresses,
        reserved_account_keys: bank.get_reserved_account_keys().clone(),
        accounts,
        feature_set: (*bank.feature_set).clone(),
        epoch_stake,
        fee: transaction.meta.fee,
        recorded_status: transaction.meta.status.clone(),
    }
}
