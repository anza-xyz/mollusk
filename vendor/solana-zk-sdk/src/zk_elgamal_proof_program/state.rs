use {
    crate::zk_elgamal_proof_program::proof_data::{pod::PodProofType, ProofType},
    bytemuck::{bytes_of, Pod, Zeroable},
    num_traits::ToPrimitive,
    solana_instruction::error::{InstructionError, InstructionError::InvalidAccountData},
    solana_pubkey::Pubkey,
    std::mem::size_of,
};

/// The on-chain state for a verified zero-knowledge proof statement.
///
/// In a zero-knowledge proof system, there is a distinction between a **proof** and a
/// **statement**.
/// - The **statement** consists of the public values that a proof is certifying. For example, in a
///   `VerifyZeroCiphertext` instruction, the statement is the ElGamal ciphertext itself.
/// - The **proof** is the cryptographic data that demonstrates the statement's validity without
///   revealing any secret information.
///
/// A proof is ephemeral and is discarded after it is successfully verified by a proof
/// instruction. However, the instruction can optionally store the verified public statement
/// on-chain in a dedicated account. The `ProofContextState` struct defines the layout of this
/// account.
///
/// Storing the statement on-chain acts as a verifiable receipt or certificate that a specific
/// proof was successfully processed. This state can then be referenced by other on-chain programs.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct ProofContextState<T: Pod> {
    /// The proof context authority that can close the account
    pub context_state_authority: Pubkey,
    /// The proof type for the context data
    pub proof_type: PodProofType,
    /// The proof context data
    pub proof_context: T,
}

// `bytemuck::Pod` cannot be derived for generic structs unless the struct is marked
// `repr(packed)`, which may cause unnecessary complications when referencing its fields. Directly
// mark `ProofContextState` as `Zeroable` and `Pod` since none of its fields has an alignment
// requirement greater than 1 and therefore, guaranteed to be `packed`.
unsafe impl<T: Pod> Zeroable for ProofContextState<T> {}
unsafe impl<T: Pod> Pod for ProofContextState<T> {}

impl<T: Pod> ProofContextState<T> {
    pub fn encode(
        context_state_authority: &Pubkey,
        proof_type: ProofType,
        proof_context: &T,
    ) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        buf.extend_from_slice(context_state_authority.as_ref());
        buf.push(ToPrimitive::to_u8(&proof_type).unwrap());
        buf.extend_from_slice(bytes_of(proof_context));
        buf
    }

    /// Interpret a slice as a `ProofContextState`.
    ///
    /// This function requires a generic parameter. To access only the generic-independent fields
    /// in `ProofContextState` without a generic parameter, use
    /// `ProofContextStateMeta::try_from_bytes` instead.
    pub fn try_from_bytes(input: &[u8]) -> Result<&Self, InstructionError> {
        bytemuck::try_from_bytes(input).map_err(|_| InvalidAccountData)
    }
}

/// The `ProofContextState` without the proof context itself. This struct exists to facilitate the
/// decoding of generic-independent fields in `ProofContextState`.
#[derive(Clone, Copy, Debug, PartialEq, bytemuck_derive::Pod, bytemuck_derive::Zeroable)]
#[repr(C)]
pub struct ProofContextStateMeta {
    /// The proof context authority that can close the account
    pub context_state_authority: Pubkey,
    /// The proof type for the context data
    pub proof_type: PodProofType,
}

impl ProofContextStateMeta {
    pub fn try_from_bytes(input: &[u8]) -> Result<&Self, InstructionError> {
        input
            .get(..size_of::<ProofContextStateMeta>())
            .and_then(|data| bytemuck::try_from_bytes(data).ok())
            .ok_or(InvalidAccountData)
    }
}
