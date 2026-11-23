//! The pre-execution state of a historical transaction.

use {
    crate::error::ReplayError,
    agave_feature_set::FeatureSet,
    mollusk_svm::epoch_stake::EpochStake,
    serde::{Deserialize, Serialize},
    solana_account::Account,
    solana_clock::Slot,
    solana_message::{
        v0::LoadedAddresses, SanitizedMessage, SanitizedVersionedMessage, SimpleAddressLoader,
        VersionedMessage,
    },
    solana_pubkey::Pubkey,
    solana_transaction_error::TransactionError,
    std::{collections::HashSet, path::Path},
};

/// Everything needed to re-execute a transaction exactly as it was processed
/// on-chain.
///
/// The context is data-only. It can be built by hand, or, with the `ledger`
/// feature, loaded from an Agave ledger directory.
#[derive(Clone, Debug)]
pub struct ReplayContext {
    /// The slot in which the transaction was processed.
    pub slot: Slot,
    /// The transaction's message.
    pub message: VersionedMessage,
    /// The addresses the message loaded from address lookup tables, as
    /// recorded in the transaction's status meta.
    pub loaded_addresses: LoadedAddresses,
    /// The reserved account keys active at `slot`, which are demoted to
    /// read-only regardless of the message header.
    pub reserved_account_keys: HashSet<Pubkey>,
    /// The state of every account visible to the transaction before it was
    /// executed, prior to fee collection.
    ///
    /// This must include every account key of the message, the program data
    /// accounts of any loader-v3 programs, and all sysvar accounts.
    pub accounts: Vec<(Pubkey, Account)>,
    /// The feature set active at `slot`.
    pub feature_set: FeatureSet,
    /// The vote account stakes of the epoch containing `slot`.
    pub epoch_stake: EpochStake,
    /// The fee the transaction paid, which is deducted from the fee payer
    /// before execution.
    pub fee: u64,
    /// The execution result recorded in the ledger.
    pub recorded_status: Result<(), TransactionError>,
}

impl ReplayContext {
    /// Sanitize the message, resolving lookup table addresses from
    /// `loaded_addresses`.
    pub fn sanitized_message(&self) -> Result<SanitizedMessage, ReplayError> {
        self.sanitize_message(&self.reserved_account_keys)
    }

    /// Sanitize the message against the provided reserved account keys.
    pub(crate) fn sanitize_message(
        &self,
        reserved_account_keys: &HashSet<Pubkey>,
    ) -> Result<SanitizedMessage, ReplayError> {
        let message = SanitizedVersionedMessage::try_new(self.message.clone())
            .map_err(|err| ReplayError::InvalidMessage(err.to_string()))?;
        SanitizedMessage::try_new(
            message,
            SimpleAddressLoader::Enabled(self.loaded_addresses.clone()),
            reserved_account_keys,
        )
        .map_err(|err| ReplayError::InvalidMessage(err.to_string()))
    }

    /// Serialize the context to a cache file.
    ///
    /// Building a context from a ledger is expensive: it loads a snapshot and
    /// replays the ledger to the transaction's slot. Saving the context lets
    /// repeated replays, for example while iterating on a candidate program,
    /// skip that work by loading it back with [`ReplayContext::load`].
    pub fn save(&self, path: &Path) -> Result<(), ReplayError> {
        let data = ReplayContextData::from(self);
        let bytes = bincode::serialize(&data).map_err(|err| ReplayError::Cache(err.to_string()))?;
        std::fs::write(path, bytes).map_err(|err| ReplayError::Cache(err.to_string()))
    }

    /// Load a context previously written with [`ReplayContext::save`].
    pub fn load(path: &Path) -> Result<Self, ReplayError> {
        let bytes = std::fs::read(path).map_err(|err| ReplayError::Cache(err.to_string()))?;
        let data: ReplayContextData =
            bincode::deserialize(&bytes).map_err(|err| ReplayError::Cache(err.to_string()))?;
        Ok(data.into())
    }
}

/// A serializable view of a [`ReplayContext`].
///
/// `FeatureSet` and `EpochStake` are stored as flat lists, and the active
/// feature set is rebuilt on load, which also reconstructs the feature
/// snapshot the runtime reads.
#[derive(Serialize, Deserialize)]
struct ReplayContextData {
    slot: Slot,
    message: VersionedMessage,
    loaded_addresses: LoadedAddresses,
    reserved_account_keys: Vec<Pubkey>,
    accounts: Vec<(Pubkey, Account)>,
    active_features: Vec<(Pubkey, Slot)>,
    epoch_stake: Vec<(Pubkey, u64)>,
    fee: u64,
    recorded_status: Result<(), TransactionError>,
}

impl From<&ReplayContext> for ReplayContextData {
    fn from(context: &ReplayContext) -> Self {
        Self {
            slot: context.slot,
            message: context.message.clone(),
            loaded_addresses: context.loaded_addresses.clone(),
            reserved_account_keys: context.reserved_account_keys.iter().copied().collect(),
            accounts: context.accounts.clone(),
            active_features: context
                .feature_set
                .active()
                .iter()
                .map(|(id, slot)| (*id, *slot))
                .collect(),
            epoch_stake: context
                .epoch_stake
                .iter()
                .map(|(key, stake)| (*key, *stake))
                .collect(),
            fee: context.fee,
            recorded_status: context.recorded_status.clone(),
        }
    }
}

impl From<ReplayContextData> for ReplayContext {
    fn from(data: ReplayContextData) -> Self {
        let mut feature_set = FeatureSet::default();
        for (id, slot) in &data.active_features {
            feature_set.activate(id, *slot);
        }
        ReplayContext {
            slot: data.slot,
            message: data.message,
            loaded_addresses: data.loaded_addresses,
            reserved_account_keys: data.reserved_account_keys.into_iter().collect(),
            accounts: data.accounts,
            feature_set,
            epoch_stake: data.epoch_stake.into_iter().collect(),
            fee: data.fee,
            recorded_status: data.recorded_status,
        }
    }
}
