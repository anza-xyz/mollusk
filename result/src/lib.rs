//! SVM program execution results and validation.
//!
//! This crate provides types and utilities for working with the results of
//! SVM program execution, including validation and comparison capabilities.
//!
//! # Core Types
//!
//! * [`InstructionResult`] - The main result type containing execution details
//! * [`ProgramResult`] - The program's execution outcome (success/failure)
//! * [`ContextResult`] - Result type for use with `MolluskContext`
//!
//! # Validation
//!
//! * [`Check`] - Validate individual instruction results
//! * [`Compare`] - Compare two instruction results
//! * [`Config`] - Configuration for validation behavior
//! * [`CheckContext`] - Context trait for custom validation logic
//!
//! # Example
//!
//! ```rust,ignore
//! use mollusk_svm_result::{Check, Config, InstructionResult};
//!
//! let result = InstructionResult::default();
//! let checks = vec![Check::success(), Check::compute_units(100)];
//! let config = Config::default();
//!
//! result.run_checks(&checks, &config, &mollusk);
//! ```

pub mod check;
pub mod compare;
pub mod config;
pub mod fuzz;
pub mod types;

// Re-export the main types and traits for convenience, and for backwards
// compatibility.
pub use {
    check::{AccountCheckBuilder, Check},
    compare::Compare,
    config::{CheckContext, Config},
    types::{ContextResult, InstructionResult, ProgramResult},
};
