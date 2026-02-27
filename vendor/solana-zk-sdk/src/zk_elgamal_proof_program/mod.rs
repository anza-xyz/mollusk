//! The native ZK ElGamal proof program.
//!
//! The program verifies a number of zero-knowledge proofs that are tailored to work with Pedersen
//! commitments and ElGamal encryption over the elliptic curve curve25519. A general overview of
//! the program as well as the technical details of some of the proof instructions can be found in
//! the [`ZK ElGamal proof`] documentation.
//!
//! This module contains the instruction types, the proof context state, and the proof instruction
//! data types. For the precise processor logic, see the ZK ElGamal Proof [`program`].
//! implementation.
//!
//! [`ZK ElGamal proof`]: https://docs.solanalabs.com/runtime/zk-token-proof
//! [`program`]: https://github.com/anza-xyz/agave/blob/master/programs/zk-elgamal-proof/src/lib.rs

pub mod errors;
pub mod instruction;
pub mod proof_data;
pub mod state;

// Program Id of the ZK ElGamal Proof program
pub use solana_sdk_ids::zk_elgamal_proof_program::{check_id, id, ID};
