//! Replay historical transactions against user-provided program bytes.
//!
//! Given the pre-execution state of a transaction that already landed
//! on-chain, Mollusk can re-execute it exactly as it was processed, then
//! execute it again with one of its programs swapped for a candidate build.
//!
//! The workflow is two-phased:
//!
//! 1. **Sanity check:** the transaction is replayed against the historical
//!    program bytes, and its execution result must match the one recorded in
//!    the ledger. This smokes out any discrepancy between the captured state
//!    and the cluster before any conclusions are drawn about the candidate.
//! 2. **Candidate run:** the transaction is replayed again with the candidate
//!    program bytes, and its execution result is compared against the recorded
//!    one.
//!
//! ```rust,ignore
//! use mollusk_svm_replay::Replayer;
//!
//! let context = ReplayContext::from_ledger(&ledger_path, &signature, &config)?;
//! let report = Replayer::new(context)?.verify(&program_id, &elf)?;
//! assert!(report.candidate_matches());
//! ```
//!
//! A `ReplayContext` can be built by hand, or, with the `ledger` feature,
//! loaded from an Agave ledger directory with `ReplayContext::from_ledger`.

mod context;
mod error;
#[cfg(feature = "ledger")]
pub mod ledger;
#[cfg(feature = "ledger")]
mod progress;
mod replayer;

pub use {
    context::ReplayContext,
    error::ReplayError,
    replayer::{execution_result, ReplayReport, Replayer},
};
