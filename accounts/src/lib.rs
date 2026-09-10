//! Account setup helpers for Mollusk.
//!
//! Each helper is offered through a builder pattern, with methods for the
//! presumably most common fields tests might set or manipulate.
//!
//! Helpers can be built directly from library payloads via `from_state`.
//!
//! ```ignore
//! let alice = Pubkey::new_unique();
//! let alice_account = System::new(alice);
//! let bob = Pubkey::new_unique();
//! let bob_account = System::new(bob).lamports(1_000);
//! ```
//!
//! ```ignore
//! let alice = Pubkey::new_unique();
//! let bob = Pubkey::new_unique();
//!
//! mollusk.process_instruction(
//!     &instruction,
//!     &[
//!         System::new(alice),
//!         System::new(bob).lamports(1_000),
//!     ],
//! );
//! ```

mod lamports;
mod stake;
mod system;
mod token;

pub use {
    stake::Stake,
    system::System,
    token::{Mint, TokenAccount},
};
