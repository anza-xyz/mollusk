//! Replay a historical transaction from a ledger against a candidate program.

use {
    clap::Args,
    mollusk_svm::result::types::TransactionResult,
    mollusk_svm_replay::{execution_result, ledger::LedgerConfig, ReplayContext, Replayer},
    solana_account::Account,
    solana_pubkey::Pubkey,
    solana_signature::Signature,
    std::{
        collections::HashMap,
        error::Error,
        fs::File,
        io::BufReader,
        path::{Path, PathBuf},
        str::FromStr,
    },
};

/// The blockstore directory the runtime opens inside the ledger.
const ROCKSDB_DIR: &str = "rocksdb";
/// The compressed blockstore as downloaded by the snapshot tool.
const ROCKSDB_ARCHIVE: &str = "rocksdb.tar.zst";

#[derive(Args)]
pub struct ReplayArgs {
    /// Path to the Agave ledger directory containing the transaction. Not
    /// needed when loading a cached context with `--context`.
    #[arg(long, required_unless_present = "context")]
    ledger: Option<PathBuf>,
    /// Signature of the transaction to replay. Not needed when loading a cached
    /// context with `--context`.
    #[arg(long, value_parser = Signature::from_str, required_unless_present = "context")]
    signature: Option<Signature>,
    /// The ID of the program to swap for the candidate ELF.
    #[arg(long, value_parser = Pubkey::from_str)]
    program_id: Pubkey,
    /// Path to the candidate program ELF.
    #[arg(long)]
    elf: PathBuf,
    /// Load the replay context from a cache file written by `--save-context`,
    /// instead of rebuilding it from the ledger. Skips the snapshot load.
    #[arg(long, conflicts_with_all = ["ledger", "signature"])]
    context: Option<PathBuf>,
    /// After building the context from the ledger, save it to this file so
    /// later replays can load it with `--context` and skip the snapshot load.
    #[arg(long, conflicts_with = "context")]
    save_context: Option<PathBuf>,
    /// Print the reconstructed pre-state (each message account's owner and
    /// size) and the instructions, then continue.
    #[arg(long)]
    dump_context: bool,
    /// Path to the genesis to load: a `genesis.bin` file, or a directory
    /// containing one. Defaults to the ledger directory.
    #[arg(long)]
    genesis: Option<PathBuf>,
    /// Directory containing the snapshot archives to load from. Defaults to
    /// the ledger directory.
    #[arg(long)]
    snapshot_archives_dir: Option<PathBuf>,
    /// Directory accounts are unpacked into. Defaults to
    /// `<ledger>/mollusk-replay`.
    #[arg(long)]
    work_dir: Option<PathBuf>,
    /// A feature to activate for the candidate run, on top of those active at
    /// the transaction's slot. The sanity check always uses the historical
    /// feature set. May be repeated.
    #[arg(long = "activate-feature", value_parser = Pubkey::from_str)]
    activate_features: Vec<Pubkey>,
    /// Enable emission of program logs to stdout. Disabled by default.
    #[arg(long)]
    program_logs: bool,
}

pub fn run(args: ReplayArgs) -> Result<(), Box<dyn Error>> {
    // Mollusk resets the logger whenever an instance is created, but always
    // honors `RUST_LOG`, so the filter is set through the environment.
    if std::env::var_os("RUST_LOG").is_none() {
        let filter = if args.program_logs {
            "solana_runtime::message_processor=debug"
        } else {
            "error"
        };
        std::env::set_var("RUST_LOG", filter);
    }
    solana_logger::setup_with_default("error");

    let elf = mollusk_svm::file::read_file(&args.elf);

    let context = if let Some(path) = &args.context {
        eprintln!("Loading cached context from {}", path.display());
        ReplayContext::load(path)?
    } else {
        // `required_unless_present` guarantees both are set without `--context`.
        let ledger = args.ledger.as_deref().expect("ledger is required");
        let signature = args.signature.expect("signature is required");

        eprintln!("Replaying transaction {signature}");
        ensure_blockstore_extracted(ledger)?;

        let context = ReplayContext::from_ledger(
            ledger,
            &signature,
            &LedgerConfig {
                genesis: args.genesis,
                snapshot_archives_dir: args.snapshot_archives_dir,
                work_dir: args.work_dir,
            },
        )?;
        if let Some(path) = &args.save_context {
            context.save(path)?;
            eprintln!("  cached context saved to {}", path.display());
        }
        context
    };
    let slot = context.slot;

    if args.dump_context {
        dump_context(&context);
    }

    let mut replayer = Replayer::new(context)?;
    if !args.activate_features.is_empty() {
        replayer.activate_candidate_features(&args.activate_features)?;
    }
    let report = replayer.verify(&args.program_id, &elf)?;

    println!();
    println!("{}", style::bold(&format!("Replay report · slot {slot}")));
    println!("  {}  {}", label("recorded"), fmt_status(&report.recorded));
    println!("  {}  {}", label("baseline"), fmt_result(&report.baseline));
    println!("  {}  {}", label("candidate"), fmt_result(&report.candidate));
    for feature_id in &args.activate_features {
        println!("  {}", style::dim(&format!("           + feature {feature_id} activated")));
    }
    println!();

    if report.candidate_matches() {
        println!(
            "  {} candidate matches the recorded result",
            style::green("✓")
        );
        Ok(())
    } else {
        println!(
            "  {} candidate does not match the recorded result",
            style::red("✗")
        );
        Err("candidate result does not match the recorded result".into())
    }
}

/// Unpack a packaged blockstore in place.
///
/// The snapshot tool downloads the blockstore as a single `rocksdb.tar.zst`,
/// but the runtime opens an unpacked `rocksdb/` directory. If the ledger holds
/// the archive and not the directory, extract it. The archive unpacks to a
/// top-level `rocksdb/`, so it is extracted into the ledger directory itself.
///
/// This is a no-op once `rocksdb/` exists, and leaves the archive in place.
fn ensure_blockstore_extracted(ledger: &Path) -> Result<(), Box<dyn Error>> {
    if ledger.join(ROCKSDB_DIR).is_dir() {
        return Ok(());
    }

    let archive = ledger.join(ROCKSDB_ARCHIVE);
    if !archive.is_file() {
        // Nothing to unpack. Let the blockstore surface the missing directory.
        return Ok(());
    }

    eprintln!(
        "  unpacking {} (one-time step, can take a while)...",
        archive.display()
    );
    let decoder = zstd::Decoder::new(BufReader::new(File::open(&archive)?))?;
    tar::Archive::new(decoder).unpack(ledger)?;

    if !ledger.join(ROCKSDB_DIR).is_dir() {
        return Err(format!("{} did not contain a `rocksdb/` directory", archive.display()).into());
    }
    Ok(())
}

/// Print the reconstructed pre-state and the transaction's instructions.
///
/// Each message account is shown with the owner it was captured with. An
/// account the message references but the context does not hold is flagged: the
/// replay stubs it as a default (System-owned) account, which is a common cause
/// of a sanity-check `InvalidAccountOwner`.
fn dump_context(context: &ReplayContext) {
    let message = match context.sanitized_message() {
        Ok(message) => message,
        Err(err) => {
            eprintln!("could not sanitize message for dump: {err}");
            return;
        }
    };

    let accounts: HashMap<&Pubkey, &Account> =
        context.accounts.iter().map(|(key, a)| (key, a)).collect();
    let keys = message.account_keys();

    println!(
        "--- context @ slot {} ({} captured accounts, {} message keys) ---",
        context.slot,
        context.accounts.len(),
        keys.len(),
    );
    for (index, key) in keys.iter().enumerate() {
        match accounts.get(key) {
            Some(account) => println!(
                "  [{index:2}] {key} owner={} lamports={} exec={} data={}B",
                account.owner,
                account.lamports,
                account.executable,
                account.data.len(),
            ),
            None => println!("  [{index:2}] {key} <NOT CAPTURED - stubbed as System-owned>"),
        }
    }
    for (index, instruction) in message.instructions().iter().enumerate() {
        let program = keys
            .get(instruction.program_id_index as usize)
            .map(Pubkey::to_string)
            .unwrap_or_else(|| "?".to_owned());
        println!(
            "  ix {index}: program {program} account_indices {:?}",
            instruction.accounts,
        );
    }
    println!("--- end context ---");
}

/// A dim, fixed-width row label so the status column lines up.
fn label(text: &str) -> String {
    style::dim(&format!("{text:<9}"))
}

/// Format an execution status as a green `ok` or a red `err · <error>`.
fn fmt_status<E: std::fmt::Display>(status: &Result<(), E>) -> String {
    match status {
        Ok(()) => style::green("ok"),
        Err(err) => style::red(&format!("err · {err}")),
    }
}

/// Format a replay result: its status followed by the compute units consumed.
fn fmt_result(result: &TransactionResult) -> String {
    format!(
        "{}  {}",
        fmt_status(&execution_result(result)),
        style::dim(&format!("{} CU", group_thousands(result.compute_units_consumed))),
    )
}

/// Render an integer with thousands separators, e.g. `344968` -> `344,968`.
fn group_thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.char_indices() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Minimal ANSI styling for the report, enabled only when stdout is a terminal
/// and `NO_COLOR` is unset.
mod style {
    use std::{io::IsTerminal, sync::OnceLock};

    fn enabled() -> bool {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        *ENABLED.get_or_init(|| {
            std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
        })
    }

    fn paint(code: &str, text: &str) -> String {
        if enabled() {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_owned()
        }
    }

    pub fn bold(text: &str) -> String {
        paint("1", text)
    }

    pub fn dim(text: &str) -> String {
        paint("2", text)
    }

    pub fn green(text: &str) -> String {
        paint("32", text)
    }

    pub fn red(text: &str) -> String {
        paint("31", text)
    }
}
