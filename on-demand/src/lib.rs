//! # Mollusk On-Demand
//!
//! Simplify Solana program testing with Mollusk by automatically fetching mainnet accounts.
//!
//! This crate provides utility functions for testing Solana programs with real mainnet state,
//! built on top of [Mollusk](https://github.com/buffalojoec/mollusk). The functions automatically
//! fetch accounts from RPC and execute using Mollusk's `MolluskContext`.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use mollusk_on_demand::process_instruction_with_context;
//! use mollusk_svm::Mollusk;
//!
//! #[tokio::test]
//! async fn test_with_mainnet_accounts() -> Result<(), Box<dyn std::error::Error>> {
//!     let mollusk = Mollusk::new(&program_id, "program_name");
//!
//!     // Process instruction - accounts are fetched automatically!
//!     let result = process_instruction_with_context(
//!         "https://api.mainnet-beta.solana.com",
//!         mollusk,
//!         &instruction,
//!         &[]
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod account_store;

use {
    account_store::{RpcAccountStore, RpcError},
    mollusk_svm::{result::InstructionResult, Mollusk},
    solana_account::Account,
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
};

/// Result type for the harness functions.
pub type Result<T> = std::result::Result<T, RpcError>;

// =============================================================================
// Permissive variants (allow missing accounts, skip validation)
// =============================================================================

/// Process an instruction with automatic account fetching (permissive mode).
///
/// This variant:
/// - Allows missing accounts (creates default empty accounts)
/// - Skips ELF validation
///
/// # Example
///
/// ```rust,ignore
/// let mollusk = Mollusk::new(&program_id, "program_name");
/// let result = process_instruction_with_context(
///     "https://api.mainnet-beta.solana.com",
///     mollusk,
///     &instruction,
///     &[(pubkey, account)]  // Optional: provide mocked accounts
/// ).await?;
/// ```
pub async fn process_instruction_with_context(
    rpc_url: &str,
    mut mollusk: Mollusk,
    instruction: &Instruction,
    accounts: &[(Pubkey, Account)],
) -> Result<InstructionResult> {
    let cache = RpcAccountStore::new(rpc_url)
        .allow_missing_accounts()
        .skip_program_validation()
        .with_accounts(accounts)
        .from_instruction(instruction)
        .await?
        .add_programs(&mut mollusk)
        .await?
        .cache;

    let context = mollusk.with_context(cache);
    Ok(context.process_instruction(instruction))
}

/// Process a chain of instructions with automatic account fetching (permissive mode).
///
/// # Example
///
/// ```rust,ignore
/// let result = process_instruction_chain_with_context(
///     rpc_url,
///     mollusk,
///     &[ix1, ix2, ix3],
///     &[]
/// ).await?;
/// ```
pub async fn process_instruction_chain_with_context(
    rpc_url: &str,
    mut mollusk: Mollusk,
    instructions: &[Instruction],
    accounts: &[(Pubkey, Account)],
) -> Result<InstructionResult> {
    let cache = RpcAccountStore::new(rpc_url)
        .allow_missing_accounts()
        .skip_program_validation()
        .with_accounts(accounts)
        .from_instructions(instructions)
        .await?
        .add_programs(&mut mollusk)
        .await?
        .cache;

    let context = mollusk.with_context(cache);
    Ok(context.process_instruction_chain(instructions))
}

/// Process an instruction with validation checks (permissive mode).
///
/// # Example
///
/// ```rust,ignore
/// use mollusk_svm::result::Check;
///
/// let checks = vec![Check::success()];
/// process_and_validate_instruction_with_context(
///     rpc_url,
///     mollusk,
///     &instruction,
///     &[],
///     &checks
/// ).await?;
/// ```
pub async fn process_and_validate_instruction_with_context(
    rpc_url: &str,
    mut mollusk: Mollusk,
    instruction: &Instruction,
    accounts: &[(Pubkey, Account)],
    checks: &[mollusk_svm::result::Check<'_>],
) -> Result<InstructionResult> {
    let cache = RpcAccountStore::new(rpc_url)
        .allow_missing_accounts()
        .skip_program_validation()
        .with_accounts(accounts)
        .from_instruction(instruction)
        .await?
        .add_programs(&mut mollusk)
        .await?
        .cache;

    let context = mollusk.with_context(cache);
    Ok(context.process_and_validate_instruction(instruction, checks))
}

/// Process a chain of instructions with validation checks (permissive mode).
///
/// # Example
///
/// ```rust,ignore
/// use mollusk_svm::result::Check;
///
/// let checks1 = vec![Check::success()];
/// let checks2 = vec![Check::success()];
///
/// process_and_validate_instruction_chain_with_context(
///     rpc_url,
///     mollusk,
///     &[(&ix1, &checks1[..]), (&ix2, &checks2[..])],
///     &[]
/// ).await?;
/// ```
pub async fn process_and_validate_instruction_chain_with_context(
    rpc_url: &str,
    mut mollusk: Mollusk,
    instructions: &[(&Instruction, &[mollusk_svm::result::Check<'_>])],
    accounts: &[(Pubkey, Account)],
) -> Result<InstructionResult> {
    let instructions_only: Vec<_> = instructions.iter().map(|(ix, _)| (*ix).clone()).collect();

    let cache = RpcAccountStore::new(rpc_url)
        .allow_missing_accounts()
        .skip_program_validation()
        .with_accounts(accounts)
        .from_instructions(&instructions_only)
        .await?
        .add_programs(&mut mollusk)
        .await?
        .cache;

    let context = mollusk.with_context(cache);
    Ok(context.process_and_validate_instruction_chain(instructions))
}

// =============================================================================
// Strict variants (error on missing accounts, validate ELF)
// =============================================================================

/// Process an instruction with automatic account fetching (strict mode).
///
/// This variant:
/// - Errors if any account is missing
/// - Validates program ELF headers
///
/// # Example
///
/// ```rust,ignore
/// let result = process_instruction_with_context_strict(
///     rpc_url,
///     mollusk,
///     &instruction,
///     &[]
/// ).await?;
/// ```
pub async fn process_instruction_with_context_strict(
    rpc_url: &str,
    mut mollusk: Mollusk,
    instruction: &Instruction,
    accounts: &[(Pubkey, Account)],
) -> Result<InstructionResult> {
    let cache = RpcAccountStore::new(rpc_url)
        .with_accounts(accounts)
        .from_instruction(instruction)
        .await?
        .add_programs(&mut mollusk)
        .await?
        .cache;

    let context = mollusk.with_context(cache);
    Ok(context.process_instruction(instruction))
}

/// Process a chain of instructions with automatic account fetching (strict mode).
///
/// # Example
///
/// ```rust,ignore
/// let result = process_instruction_chain_with_context_strict(
///     rpc_url,
///     mollusk,
///     &[ix1, ix2, ix3],
///     &[]
/// ).await?;
/// ```
pub async fn process_instruction_chain_with_context_strict(
    rpc_url: &str,
    mut mollusk: Mollusk,
    instructions: &[Instruction],
    accounts: &[(Pubkey, Account)],
) -> Result<InstructionResult> {
    let cache = RpcAccountStore::new(rpc_url)
        .with_accounts(accounts)
        .from_instructions(instructions)
        .await?
        .add_programs(&mut mollusk)
        .await?
        .cache;

    let context = mollusk.with_context(cache);
    Ok(context.process_instruction_chain(instructions))
}

/// Process an instruction with validation checks (strict mode).
///
/// # Example
///
/// ```rust,ignore
/// use mollusk_svm::result::Check;
///
/// let checks = vec![Check::success()];
/// process_and_validate_instruction_with_context_strict(
///     rpc_url,
///     mollusk,
///     &instruction,
///     &[],
///     &checks
/// ).await?;
/// ```
pub async fn process_and_validate_instruction_with_context_strict(
    rpc_url: &str,
    mut mollusk: Mollusk,
    instruction: &Instruction,
    accounts: &[(Pubkey, Account)],
    checks: &[mollusk_svm::result::Check<'_>],
) -> Result<InstructionResult> {
    let cache = RpcAccountStore::new(rpc_url)
        .with_accounts(accounts)
        .from_instruction(instruction)
        .await?
        .add_programs(&mut mollusk)
        .await?
        .cache;

    let context = mollusk.with_context(cache);
    Ok(context.process_and_validate_instruction(instruction, checks))
}

/// Process a chain of instructions with validation checks (strict mode).
///
/// # Example
///
/// ```rust,ignore
/// use mollusk_svm::result::Check;
///
/// process_and_validate_instruction_chain_with_context_strict(
///     rpc_url,
///     mollusk,
///     &[(&ix1, &checks1[..]), (&ix2, &checks2[..])],
///     &[]
/// ).await?;
/// ```
pub async fn process_and_validate_instruction_chain_with_context_strict(
    rpc_url: &str,
    mut mollusk: Mollusk,
    instructions: &[(&Instruction, &[mollusk_svm::result::Check<'_>])],
    accounts: &[(Pubkey, Account)],
) -> Result<InstructionResult> {
    let instructions_only: Vec<_> = instructions.iter().map(|(ix, _)| (*ix).clone()).collect();

    let cache = RpcAccountStore::new(rpc_url)
        .with_accounts(accounts)
        .from_instructions(&instructions_only)
        .await?
        .add_programs(&mut mollusk)
        .await?
        .cache;

    let context = mollusk.with_context(cache);
    Ok(context.process_and_validate_instruction_chain(instructions))
}
