//! Mollusk's implementation of the SVM's invoke context callback.

#[cfg(feature = "precompiles")]
use solana_precompile_error::PrecompileError;
use {
    crate::epoch_stake::EpochStake, solana_pubkey::Pubkey,
    solana_svm_callback::InvokeContextCallback, solana_svm_feature_set::SVMFeatureSet,
};

pub(crate) struct MolluskInvokeContextCallback<'a> {
    #[cfg_attr(not(feature = "precompiles"), allow(dead_code))]
    pub feature_set: &'a SVMFeatureSet,
    pub epoch_stake: &'a EpochStake,
}

impl InvokeContextCallback for MolluskInvokeContextCallback<'_> {
    fn get_epoch_stake(&self) -> u64 {
        self.epoch_stake.values().sum()
    }

    fn get_epoch_stake_for_vote_account(&self, vote_address: &Pubkey) -> u64 {
        self.epoch_stake.get(vote_address).copied().unwrap_or(0)
    }

    #[cfg(feature = "precompiles")]
    fn is_precompile(&self, program_id: &Pubkey) -> bool {
        // `agave-precompiles` only gates `secp256r1` behind a feature, and
        // Mollusk exposes no way to toggle it off, so treat every precompile
        // as enabled.
        agave_precompiles::is_precompile(program_id, |_feature_id| true)
    }

    #[cfg(not(feature = "precompiles"))]
    fn is_precompile(&self, _program_id: &Pubkey) -> bool {
        false
    }

    #[cfg(feature = "precompiles")]
    fn process_precompile(
        &self,
        program_id: &Pubkey,
        data: &[u8],
        instruction_datas: Vec<&[u8]>,
    ) -> Result<(), PrecompileError> {
        // `agave-precompiles` only gates `secp256r1` behind a feature, and
        // Mollusk exposes no way to toggle it off, so treat every precompile
        // as enabled.
        if let Some(precompile) = agave_precompiles::get_precompile(program_id, |_feature_id| true)
        {
            // However, the Agave FeatureSet is still required as an arg here.
            let feature_set = crate::feature_set::svm_feature_set_to_feature_set(self.feature_set);
            precompile.verify(data, &instruction_datas, &feature_set)
        } else {
            Err(PrecompileError::InvalidPublicKey)
        }
    }

    #[cfg(not(feature = "precompiles"))]
    fn process_precompile(
        &self,
        _program_id: &Pubkey,
        _data: &[u8],
        _instruction_datas: Vec<&[u8]>,
    ) -> Result<(), solana_precompile_error::PrecompileError> {
        panic!("precompiles feature not enabled");
    }
}
