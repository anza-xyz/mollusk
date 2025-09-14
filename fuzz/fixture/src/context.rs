//! All test environment inputs for an instruction.

use {
    crate::{
        proto::{InstrAcct as ProtoInstructionAccount, InstrContext as ProtoContext},
        sysvars::Sysvars,
    },
    agave_feature_set::FeatureSet,
    solana_account::Account,
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_instruction::AccountMeta,
    solana_keccak_hasher::Hasher,
    solana_pubkey::Pubkey,
};

/// Instruction context fixture.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The compute budget to use for the simulation.
    pub compute_budget: ComputeBudget,
    /// The feature set to use for the simulation.
    pub feature_set: FeatureSet,
    /// The runtime sysvars to use for the simulation.
    pub sysvars: Sysvars,
    /// The program ID of the program being invoked.
    pub program_id: Pubkey,
    /// Accounts to pass to the instruction.
    pub instruction_accounts: Vec<AccountMeta>,
    /// The instruction data.
    pub instruction_data: Vec<u8>,
    /// Input accounts with state.
    pub accounts: Vec<(Pubkey, Account)>,
}

impl From<ProtoContext> for Context {
    fn from(value: ProtoContext) -> Self {
        let program_id_bytes: [u8; 32] = value
            .program_id
            .try_into()
            .expect("Invalid bytes for program ID");
        let program_id = Pubkey::new_from_array(program_id_bytes);

        let accounts: Vec<(Pubkey, Account)> = value.accounts.into_iter().map(Into::into).collect();

        let instruction_accounts: Vec<AccountMeta> = value
            .instr_accounts
            .into_iter()
            .map(
                |ProtoInstructionAccount {
                     index,
                     is_signer,
                     is_writable,
                 }| {
                    let (pubkey, _) = accounts
                        .get(index as usize)
                        .expect("Invalid index for instruction account");
                    AccountMeta {
                        pubkey: *pubkey,
                        is_signer,
                        is_writable,
                    }
                },
            )
            .collect();

        let feature_set: FeatureSet = value.feature_set.map(Into::into).unwrap_or_default();
        let simd_0268_active =
            feature_set.is_active(&agave_feature_set::raise_cpi_nesting_limit_to_8::id());

        let compute_budget = value
            .compute_budget
            .map(Into::into)
            .unwrap_or_else(|| ComputeBudget::new_with_defaults(simd_0268_active));

        Self {
            compute_budget,
            feature_set,
            sysvars: value.sysvars.map(Into::into).unwrap_or_default(),
            program_id,
            instruction_accounts,
            instruction_data: value.data,
            accounts,
        }
    }
}

impl From<Context> for ProtoContext {
    fn from(value: Context) -> Self {
        let instr_accounts: Vec<ProtoInstructionAccount> = value
            .instruction_accounts
            .into_iter()
            .map(
                |AccountMeta {
                     pubkey,
                     is_signer,
                     is_writable,
                 }| {
                    let index_of_account = value
                        .accounts
                        .iter()
                        .position(|(key, _)| key == &pubkey)
                        .unwrap();
                    ProtoInstructionAccount {
                        index: index_of_account as u32,
                        is_signer,
                        is_writable,
                    }
                },
            )
            .collect();

        let accounts = value.accounts.into_iter().map(Into::into).collect();

        Self {
            compute_budget: Some(value.compute_budget.into()),
            feature_set: Some(value.feature_set.into()),
            sysvars: Some(value.sysvars.into()),
            program_id: value.program_id.to_bytes().to_vec(),
            instr_accounts,
            data: value.instruction_data,
            accounts,
        }
    }
}

pub(crate) fn hash_proto_context(hasher: &mut Hasher, context: &ProtoContext) {
    if let Some(compute_budget) = &context.compute_budget {
        crate::compute_budget::hash_proto_compute_budget(hasher, compute_budget);
    }
    if let Some(feature_set) = &context.feature_set {
        crate::feature_set::hash_proto_feature_set(hasher, feature_set);
    }
    if let Some(sysvars) = &context.sysvars {
        crate::sysvars::hash_proto_sysvars(hasher, sysvars);
    }
    hasher.hash(&context.program_id);
    for account in context.instr_accounts.iter() {
        hasher.hash(&account.index.to_le_bytes());
        hasher.hash(&[account.is_signer as u8]);
        hasher.hash(&[account.is_writable as u8]);
    }
    hasher.hash(&context.data);
    crate::account::hash_proto_accounts(hasher, &context.accounts);
}
