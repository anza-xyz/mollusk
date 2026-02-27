//!
//! This crate contians three macros used to simulate eager macro expansion:
//!
//! 1. `eager!`: Eagerly expands any macro in its body.
//! 2. `eager_macro_rules!`: Used to declare macro that can be eagerly expanded with `eager!`.
//! 3. `lazy!`: Used in `eager!` to revert to lazy macro expansion.
//!
//! See the each macro's documentation for details.
//!
//!

#[macro_use]
mod eager;
#[macro_use]
mod eager_macro_rules;
#[macro_use]
mod lazy;
