//! Errors returned while replaying a transaction.

use {solana_pubkey::Pubkey, solana_transaction_error::TransactionError, thiserror::Error};

#[derive(Debug, Error)]
pub enum ReplayError {
    /// The replay against the historical program bytes did not reproduce the
    /// execution result recorded in the ledger.
    #[error("sanity check failed: recorded result {recorded:?}, replayed result {replayed:?}")]
    SanityCheckFailed {
        recorded: Result<(), TransactionError>,
        replayed: Result<(), TransactionError>,
    },
    /// The message could not be sanitized.
    #[error("invalid message: {0}")]
    InvalidMessage(String),
    /// The candidate program is not one of the programs loaded by the
    /// transaction.
    #[error("program {0} is not loaded by the transaction")]
    ProgramNotInTransaction(Pubkey),
    /// The transaction uses a native program Mollusk does not provide.
    #[error("unsupported builtin program {0}")]
    UnsupportedBuiltin(Pubkey),
    /// The transaction uses a program owned by a loader Mollusk does not
    /// support.
    #[error("program {program_id} is owned by unsupported loader {loader}")]
    UnsupportedLoader { program_id: Pubkey, loader: Pubkey },
    /// The feature ID is not a known feature.
    #[error("unknown feature {0}")]
    UnknownFeature(Pubkey),
    /// The transaction uses a durable nonce, whose advancement is not
    /// emulated.
    #[error("durable nonce transactions are not supported")]
    UnsupportedDurableNonce,
    /// The recorded transaction never reached program execution, so there is
    /// nothing to replay.
    #[error("transaction was not executed: {0}")]
    TransactionNotExecuted(TransactionError),
    /// The account state could not be decoded.
    #[error("invalid account {address}: {reason}")]
    InvalidAccount { address: Pubkey, reason: String },
    /// A replay context could not be saved to or loaded from a cache file.
    #[error("context cache: {0}")]
    Cache(String),
    /// The pre-execution state could not be loaded from the ledger.
    #[cfg(feature = "ledger")]
    #[error("ledger: {0}")]
    Ledger(String),
}
