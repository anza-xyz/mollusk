//! Post-invocation effects of an instruction.

use {
    super::proto::InstrEffects as ProtoEffects, crate::account::SeedAddress,
    solana_account::Account, solana_keccak_hasher::Hasher, solana_pubkey::Pubkey,
};

/// Represents the effects of a single instruction.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Effects {
    // Program return code. Zero is success, errors are non-zero.
    pub program_result: i32,
    // Custom error code, also non-zero if any.
    pub program_custom_code: u32,
    /// Copies of accounts that were changed.
    pub modified_accounts: Vec<(Pubkey, Account, Option<SeedAddress>)>,
    /// Compute units available after executing the instruction.
    pub compute_units_available: u64,
    /// Instruction return data.
    pub return_data: Vec<u8>,
}

impl From<ProtoEffects> for Effects {
    fn from(value: ProtoEffects) -> Self {
        let ProtoEffects {
            result,
            custom_err,
            modified_accounts,
            cu_avail,
            return_data,
        } = value;

        let modified_accounts: Vec<(Pubkey, Account, Option<SeedAddress>)> =
            modified_accounts.into_iter().map(Into::into).collect();

        Self {
            program_result: result,
            program_custom_code: custom_err,
            modified_accounts,
            compute_units_available: cu_avail,
            return_data,
        }
    }
}

impl From<Effects> for ProtoEffects {
    fn from(value: Effects) -> Self {
        let Effects {
            program_result,
            program_custom_code,
            modified_accounts,
            compute_units_available,
            return_data,
        } = value;

        let modified_accounts = modified_accounts.into_iter().map(Into::into).collect();

        Self {
            result: program_result,
            custom_err: program_custom_code,
            modified_accounts,
            cu_avail: compute_units_available,
            return_data,
        }
    }
}

pub(crate) fn hash_proto_effects(hasher: &mut Hasher, effects: &ProtoEffects) {
    hasher.hash(&effects.result.to_le_bytes());
    hasher.hash(&effects.custom_err.to_le_bytes());
    crate::account::hash_proto_accounts(hasher, &effects.modified_accounts);
    hasher.hash(&effects.cu_avail.to_le_bytes());
    hasher.hash(&effects.return_data);
}
