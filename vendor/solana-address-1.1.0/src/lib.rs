//! Address representation for Solana.
//!
//! An address is a sequence of 32 bytes, often shown as a base58 encoded string
//! (e.g. 14grJpemFaf88c8tiVb77W7TYg2W3ir6pfkKz3YjhhZ5).

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Re-export the v2 public API.
pub use solana_address_v2::*;
