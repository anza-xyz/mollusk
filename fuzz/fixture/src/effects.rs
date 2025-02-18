//! Post-invocation effects of an instruction.

use {
    super::proto::{AcctState as ProtoAccount, InstrEffects as ProtoEffects},
    solana_account::Account,
    solana_keccak_hasher::Hasher,
    solana_pubkey::Pubkey,
};

/// Represents the effects of a single instruction.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Effects {
    /// Compute units consumed by the instruction.
    pub compute_units_consumed: u64,
    /// Execution time for instruction.
    pub execution_time: u64,
    // Program return code. Zero is success, errors are non-zero.
    pub program_result: u64,
    pub return_data: Vec<u8>,
    /// Resulting accounts with state, to be checked post-simulation.
    pub resulting_accounts: Vec<(Pubkey, Account)>,
}

impl From<ProtoEffects> for Effects {
    fn from(value: ProtoEffects) -> Self {
        let ProtoEffects {
            compute_units_consumed,
            execution_time,
            program_result,
            return_data,
            resulting_accounts,
        } = value;

        let resulting_accounts: Vec<(Pubkey, Account)> =
            resulting_accounts.into_iter().map(Into::into).collect();

        Self {
            compute_units_consumed,
            execution_time,
            program_result,
            return_data,
            resulting_accounts,
        }
    }
}

impl From<Effects> for ProtoEffects {
    fn from(value: Effects) -> Self {
        let Effects {
            compute_units_consumed,
            execution_time,
            program_result,
            return_data,
            resulting_accounts,
        } = value;

        let resulting_accounts: Vec<ProtoAccount> =
            resulting_accounts.into_iter().map(Into::into).collect();

        Self {
            compute_units_consumed,
            execution_time,
            program_result,
            return_data,
            resulting_accounts,
        }
    }
}

pub(crate) fn hash_proto_effects(hasher: &mut Hasher, effects: &ProtoEffects) {
    hasher.hash(&effects.compute_units_consumed.to_le_bytes());
    hasher.hash(&effects.execution_time.to_le_bytes());
    hasher.hash(&effects.program_result.to_le_bytes());
    crate::account::hash_proto_accounts(hasher, &effects.resulting_accounts);
}
