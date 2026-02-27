#![allow(clippy::arithmetic_side_effects)]
#![deny(clippy::wildcard_enum_match_arm)]
// Remove the following `allow` when `StakeState` is removed, required to avoid
// warnings from uses of deprecated types during trait derivations.
#![allow(deprecated)]

#[cfg(feature = "borsh")]
use borsh::{io, BorshDeserialize, BorshSchema, BorshSerialize};
use {
    crate::{
        error::StakeError,
        instruction::LockupArgs,
        stake_flags::StakeFlags,
        stake_history::{StakeHistoryEntry, StakeHistoryGetEntry},
    },
    solana_clock::{Clock, Epoch, UnixTimestamp},
    solana_instruction::error::InstructionError,
    solana_pubkey::Pubkey,
    std::collections::HashSet,
};

pub type StakeActivationStatus = StakeHistoryEntry;

// Means that no more than RATE of current effective stake may be added or subtracted per
// epoch.
pub const DEFAULT_WARMUP_COOLDOWN_RATE: f64 = 0.25;
pub const NEW_WARMUP_COOLDOWN_RATE: f64 = 0.09;
pub const DEFAULT_SLASH_PENALTY: u8 = ((5 * u8::MAX as usize) / 100) as u8;

pub fn warmup_cooldown_rate(current_epoch: Epoch, new_rate_activation_epoch: Option<Epoch>) -> f64 {
    if current_epoch < new_rate_activation_epoch.unwrap_or(u64::MAX) {
        DEFAULT_WARMUP_COOLDOWN_RATE
    } else {
        NEW_WARMUP_COOLDOWN_RATE
    }
}

#[cfg(feature = "borsh")]
macro_rules! impl_borsh_stake_state {
    ($borsh:ident) => {
        impl $borsh::BorshDeserialize for StakeState {
            fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
                let enum_value: u32 = $borsh::BorshDeserialize::deserialize_reader(reader)?;
                match enum_value {
                    0 => Ok(StakeState::Uninitialized),
                    1 => {
                        let meta: Meta = $borsh::BorshDeserialize::deserialize_reader(reader)?;
                        Ok(StakeState::Initialized(meta))
                    }
                    2 => {
                        let meta: Meta = $borsh::BorshDeserialize::deserialize_reader(reader)?;
                        let stake: Stake = $borsh::BorshDeserialize::deserialize_reader(reader)?;
                        Ok(StakeState::Stake(meta, stake))
                    }
                    3 => Ok(StakeState::RewardsPool),
                    _ => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid enum value",
                    )),
                }
            }
        }
        impl $borsh::BorshSerialize for StakeState {
            fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
                match self {
                    StakeState::Uninitialized => writer.write_all(&0u32.to_le_bytes()),
                    StakeState::Initialized(meta) => {
                        writer.write_all(&1u32.to_le_bytes())?;
                        $borsh::BorshSerialize::serialize(&meta, writer)
                    }
                    StakeState::Stake(meta, stake) => {
                        writer.write_all(&2u32.to_le_bytes())?;
                        $borsh::BorshSerialize::serialize(&meta, writer)?;
                        $borsh::BorshSerialize::serialize(&stake, writer)
                    }
                    StakeState::RewardsPool => writer.write_all(&3u32.to_le_bytes()),
                }
            }
        }
    };
}
#[cfg_attr(feature = "frozen-abi", derive(solana_frozen_abi_macro::AbiExample))]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Deserialize, serde_derive::Serialize)
)]
#[derive(Debug, Default, PartialEq, Clone, Copy)]
#[allow(clippy::large_enum_variant)]
#[deprecated(
    since = "1.17.0",
    note = "Please use `StakeStateV2` instead, and match the third `StakeFlags` field when matching `StakeStateV2::Stake` to resolve any breakage. For example, `if let StakeState::Stake(meta, stake)` becomes `if let StakeStateV2::Stake(meta, stake, _stake_flags)`."
)]
pub enum StakeState {
    #[default]
    Uninitialized,
    Initialized(Meta),
    Stake(Meta, Stake),
    RewardsPool,
}
#[cfg(feature = "borsh")]
impl_borsh_stake_state!(borsh);
impl StakeState {
    /// The fixed number of bytes used to serialize each stake account
    pub const fn size_of() -> usize {
        200 // see test_size_of
    }

    pub fn stake(&self) -> Option<Stake> {
        match self {
            Self::Stake(_meta, stake) => Some(*stake),
            Self::Uninitialized | Self::Initialized(_) | Self::RewardsPool => None,
        }
    }

    pub fn delegation(&self) -> Option<Delegation> {
        match self {
            Self::Stake(_meta, stake) => Some(stake.delegation),
            Self::Uninitialized | Self::Initialized(_) | Self::RewardsPool => None,
        }
    }

    pub fn authorized(&self) -> Option<Authorized> {
        match self {
            Self::Stake(meta, _stake) => Some(meta.authorized),
            Self::Initialized(meta) => Some(meta.authorized),
            Self::Uninitialized | Self::RewardsPool => None,
        }
    }

    pub fn lockup(&self) -> Option<Lockup> {
        self.meta().map(|meta| meta.lockup)
    }

    pub fn meta(&self) -> Option<Meta> {
        match self {
            Self::Stake(meta, _stake) => Some(*meta),
            Self::Initialized(meta) => Some(*meta),
            Self::Uninitialized | Self::RewardsPool => None,
        }
    }
}

#[cfg_attr(feature = "frozen-abi", derive(solana_frozen_abi_macro::AbiExample))]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Deserialize, serde_derive::Serialize)
)]
#[derive(Debug, Default, PartialEq, Clone, Copy)]
#[allow(clippy::large_enum_variant)]
pub enum StakeStateV2 {
    #[default]
    Uninitialized,
    Initialized(Meta),
    Stake(Meta, Stake, StakeFlags),
    RewardsPool,
}
#[cfg(feature = "borsh")]
macro_rules! impl_borsh_stake_state_v2 {
    ($borsh:ident) => {
        impl $borsh::BorshDeserialize for StakeStateV2 {
            fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
                let enum_value: u32 = $borsh::BorshDeserialize::deserialize_reader(reader)?;
                match enum_value {
                    0 => Ok(StakeStateV2::Uninitialized),
                    1 => {
                        let meta: Meta = $borsh::BorshDeserialize::deserialize_reader(reader)?;
                        Ok(StakeStateV2::Initialized(meta))
                    }
                    2 => {
                        let meta: Meta = $borsh::BorshDeserialize::deserialize_reader(reader)?;
                        let stake: Stake = $borsh::BorshDeserialize::deserialize_reader(reader)?;
                        let stake_flags: StakeFlags =
                            $borsh::BorshDeserialize::deserialize_reader(reader)?;
                        Ok(StakeStateV2::Stake(meta, stake, stake_flags))
                    }
                    3 => Ok(StakeStateV2::RewardsPool),
                    _ => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid enum value",
                    )),
                }
            }
        }
        impl $borsh::BorshSerialize for StakeStateV2 {
            fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
                match self {
                    StakeStateV2::Uninitialized => writer.write_all(&0u32.to_le_bytes()),
                    StakeStateV2::Initialized(meta) => {
                        writer.write_all(&1u32.to_le_bytes())?;
                        $borsh::BorshSerialize::serialize(&meta, writer)
                    }
                    StakeStateV2::Stake(meta, stake, stake_flags) => {
                        writer.write_all(&2u32.to_le_bytes())?;
                        $borsh::BorshSerialize::serialize(&meta, writer)?;
                        $borsh::BorshSerialize::serialize(&stake, writer)?;
                        $borsh::BorshSerialize::serialize(&stake_flags, writer)
                    }
                    StakeStateV2::RewardsPool => writer.write_all(&3u32.to_le_bytes()),
                }
            }
        }
    };
}
#[cfg(feature = "borsh")]
impl_borsh_stake_state_v2!(borsh);

impl StakeStateV2 {
    /// The fixed number of bytes used to serialize each stake account
    pub const fn size_of() -> usize {
        200 // see test_size_of
    }

    pub fn stake(&self) -> Option<Stake> {
        match self {
            Self::Stake(_meta, stake, _stake_flags) => Some(*stake),
            Self::Uninitialized | Self::Initialized(_) | Self::RewardsPool => None,
        }
    }

    pub fn stake_ref(&self) -> Option<&Stake> {
        match self {
            Self::Stake(_meta, stake, _stake_flags) => Some(stake),
            Self::Uninitialized | Self::Initialized(_) | Self::RewardsPool => None,
        }
    }

    pub fn delegation(&self) -> Option<Delegation> {
        match self {
            Self::Stake(_meta, stake, _stake_flags) => Some(stake.delegation),
            Self::Uninitialized | Self::Initialized(_) | Self::RewardsPool => None,
        }
    }

    pub fn delegation_ref(&self) -> Option<&Delegation> {
        match self {
            StakeStateV2::Stake(_meta, stake, _stake_flags) => Some(&stake.delegation),
            Self::Uninitialized | Self::Initialized(_) | Self::RewardsPool => None,
        }
    }

    pub fn authorized(&self) -> Option<Authorized> {
        match self {
            Self::Stake(meta, _stake, _stake_flags) => Some(meta.authorized),
            Self::Initialized(meta) => Some(meta.authorized),
            Self::Uninitialized | Self::RewardsPool => None,
        }
    }

    pub fn lockup(&self) -> Option<Lockup> {
        self.meta().map(|meta| meta.lockup)
    }

    pub fn meta(&self) -> Option<Meta> {
        match self {
            Self::Stake(meta, _stake, _stake_flags) => Some(*meta),
            Self::Initialized(meta) => Some(*meta),
            Self::Uninitialized | Self::RewardsPool => None,
        }
    }
}

#[cfg_attr(feature = "frozen-abi", derive(solana_frozen_abi_macro::AbiExample))]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Deserialize, serde_derive::Serialize)
)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StakeAuthorize {
    Staker,
    Withdrawer,
}

#[cfg_attr(feature = "frozen-abi", derive(solana_frozen_abi_macro::AbiExample))]
#[cfg_attr(
    feature = "borsh",
    derive(BorshSerialize, BorshDeserialize, BorshSchema),
    borsh(crate = "borsh")
)]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Deserialize, serde_derive::Serialize)
)]
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Lockup {
    /// UnixTimestamp at which this stake will allow withdrawal, unless the
    ///   transaction is signed by the custodian
    pub unix_timestamp: UnixTimestamp,
    /// epoch height at which this stake will allow withdrawal, unless the
    ///   transaction is signed by the custodian
    pub epoch: Epoch,
    /// custodian signature on a transaction exempts the operation from
    ///  lockup constraints
    pub custodian: Pubkey,
}
impl Lockup {
    pub fn is_in_force(&self, clock: &Clock, custodian: Option<&Pubkey>) -> bool {
        if custodian == Some(&self.custodian) {
            return false;
        }
        self.unix_timestamp > clock.unix_timestamp || self.epoch > clock.epoch
    }
}

#[cfg_attr(feature = "frozen-abi", derive(solana_frozen_abi_macro::AbiExample))]
#[cfg_attr(
    feature = "borsh",
    derive(BorshSerialize, BorshDeserialize, BorshSchema),
    borsh(crate = "borsh")
)]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Deserialize, serde_derive::Serialize)
)]
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Authorized {
    pub staker: Pubkey,
    pub withdrawer: Pubkey,
}

impl Authorized {
    pub fn auto(authorized: &Pubkey) -> Self {
        Self {
            staker: *authorized,
            withdrawer: *authorized,
        }
    }
    pub fn check(
        &self,
        signers: &HashSet<Pubkey>,
        stake_authorize: StakeAuthorize,
    ) -> Result<(), InstructionError> {
        let authorized_signer = match stake_authorize {
            StakeAuthorize::Staker => &self.staker,
            StakeAuthorize::Withdrawer => &self.withdrawer,
        };

        if signers.contains(authorized_signer) {
            Ok(())
        } else {
            Err(InstructionError::MissingRequiredSignature)
        }
    }

    pub fn authorize(
        &mut self,
        signers: &HashSet<Pubkey>,
        new_authorized: &Pubkey,
        stake_authorize: StakeAuthorize,
        lockup_custodian_args: Option<(&Lockup, &Clock, Option<&Pubkey>)>,
    ) -> Result<(), InstructionError> {
        match stake_authorize {
            StakeAuthorize::Staker => {
                // Allow either the staker or the withdrawer to change the staker key
                if !signers.contains(&self.staker) && !signers.contains(&self.withdrawer) {
                    return Err(InstructionError::MissingRequiredSignature);
                }
                self.staker = *new_authorized
            }
            StakeAuthorize::Withdrawer => {
                if let Some((lockup, clock, custodian)) = lockup_custodian_args {
                    if lockup.is_in_force(clock, None) {
                        match custodian {
                            None => {
                                return Err(StakeError::CustodianMissing.into());
                            }
                            Some(custodian) => {
                                if !signers.contains(custodian) {
                                    return Err(StakeError::CustodianSignatureMissing.into());
                                }

                                if lockup.is_in_force(clock, Some(custodian)) {
                                    return Err(StakeError::LockupInForce.into());
                                }
                            }
                        }
                    }
                }
                self.check(signers, stake_authorize)?;
                self.withdrawer = *new_authorized
            }
        }
        Ok(())
    }
}

#[cfg_attr(feature = "frozen-abi", derive(solana_frozen_abi_macro::AbiExample))]
#[cfg_attr(
    feature = "borsh",
    derive(BorshSerialize, BorshDeserialize, BorshSchema),
    borsh(crate = "borsh")
)]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Deserialize, serde_derive::Serialize)
)]
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Meta {
    pub rent_exempt_reserve: u64,
    pub authorized: Authorized,
    pub lockup: Lockup,
}

impl Meta {
    pub fn set_lockup(
        &mut self,
        lockup: &LockupArgs,
        signers: &HashSet<Pubkey>,
        clock: &Clock,
    ) -> Result<(), InstructionError> {
        // post-stake_program_v4 behavior:
        // * custodian can update the lockup while in force
        // * withdraw authority can set a new lockup
        if self.lockup.is_in_force(clock, None) {
            if !signers.contains(&self.lockup.custodian) {
                return Err(InstructionError::MissingRequiredSignature);
            }
        } else if !signers.contains(&self.authorized.withdrawer) {
            return Err(InstructionError::MissingRequiredSignature);
        }
        if let Some(unix_timestamp) = lockup.unix_timestamp {
            self.lockup.unix_timestamp = unix_timestamp;
        }
        if let Some(epoch) = lockup.epoch {
            self.lockup.epoch = epoch;
        }
        if let Some(custodian) = lockup.custodian {
            self.lockup.custodian = custodian;
        }
        Ok(())
    }

    pub fn auto(authorized: &Pubkey) -> Self {
        Self {
            authorized: Authorized::auto(authorized),
            ..Meta::default()
        }
    }
}

#[cfg_attr(feature = "frozen-abi", derive(solana_frozen_abi_macro::AbiExample))]
#[cfg_attr(
    feature = "borsh",
    derive(BorshSerialize, BorshDeserialize, BorshSchema),
    borsh(crate = "borsh")
)]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Deserialize, serde_derive::Serialize)
)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Delegation {
    /// to whom the stake is delegated
    pub voter_pubkey: Pubkey,
    /// activated stake amount, set at delegate() time
    pub stake: u64,
    /// epoch at which this stake was activated, `std::u64::MAX` if is a bootstrap stake
    pub activation_epoch: Epoch,
    /// epoch the stake was deactivated, `std::u64::MAX` if not deactivated
    pub deactivation_epoch: Epoch,
    /// how much stake we can activate per-epoch as a fraction of currently effective stake
    #[deprecated(
        since = "1.16.7",
        note = "Please use `solana_sdk::stake::state::warmup_cooldown_rate()` instead"
    )]
    pub warmup_cooldown_rate: f64,
}

impl Default for Delegation {
    fn default() -> Self {
        #[allow(deprecated)]
        Self {
            voter_pubkey: Pubkey::default(),
            stake: 0,
            activation_epoch: 0,
            deactivation_epoch: u64::MAX,
            warmup_cooldown_rate: DEFAULT_WARMUP_COOLDOWN_RATE,
        }
    }
}

impl Delegation {
    pub fn new(voter_pubkey: &Pubkey, stake: u64, activation_epoch: Epoch) -> Self {
        Self {
            voter_pubkey: *voter_pubkey,
            stake,
            activation_epoch,
            ..Delegation::default()
        }
    }
    pub fn is_bootstrap(&self) -> bool {
        self.activation_epoch == u64::MAX
    }

    pub fn stake<T: StakeHistoryGetEntry>(
        &self,
        epoch: Epoch,
        history: &T,
        new_rate_activation_epoch: Option<Epoch>,
    ) -> u64 {
        self.stake_activating_and_deactivating(epoch, history, new_rate_activation_epoch)
            .effective
    }

    #[allow(clippy::comparison_chain)]
    pub fn stake_activating_and_deactivating<T: StakeHistoryGetEntry>(
        &self,
        target_epoch: Epoch,
        history: &T,
        new_rate_activation_epoch: Option<Epoch>,
    ) -> StakeActivationStatus {
        // first, calculate an effective and activating stake
        let (effective_stake, activating_stake) =
            self.stake_and_activating(target_epoch, history, new_rate_activation_epoch);

        // then de-activate some portion if necessary
        if target_epoch < self.deactivation_epoch {
            // not deactivated
            if activating_stake == 0 {
                StakeActivationStatus::with_effective(effective_stake)
            } else {
                StakeActivationStatus::with_effective_and_activating(
                    effective_stake,
                    activating_stake,
                )
            }
        } else if target_epoch == self.deactivation_epoch {
            // can only deactivate what's activated
            StakeActivationStatus::with_deactivating(effective_stake)
        } else if let Some((history, mut prev_epoch, mut prev_cluster_stake)) = history
            .get_entry(self.deactivation_epoch)
            .map(|cluster_stake_at_deactivation_epoch| {
                (
                    history,
                    self.deactivation_epoch,
                    cluster_stake_at_deactivation_epoch,
                )
            })
        {
            // target_epoch > self.deactivation_epoch

            // loop from my deactivation epoch until the target epoch
            // current effective stake is updated using its previous epoch's cluster stake
            let mut current_epoch;
            let mut current_effective_stake = effective_stake;
            loop {
                current_epoch = prev_epoch + 1;
                // if there is no deactivating stake at prev epoch, we should have been
                // fully undelegated at this moment
                if prev_cluster_stake.deactivating == 0 {
                    break;
                }

                // I'm trying to get to zero, how much of the deactivation in stake
                //   this account is entitled to take
                let weight =
                    current_effective_stake as f64 / prev_cluster_stake.deactivating as f64;
                let warmup_cooldown_rate =
                    warmup_cooldown_rate(current_epoch, new_rate_activation_epoch);

                // portion of newly not-effective cluster stake I'm entitled to at current epoch
                let newly_not_effective_cluster_stake =
                    prev_cluster_stake.effective as f64 * warmup_cooldown_rate;
                let newly_not_effective_stake =
                    ((weight * newly_not_effective_cluster_stake) as u64).max(1);

                current_effective_stake =
                    current_effective_stake.saturating_sub(newly_not_effective_stake);
                if current_effective_stake == 0 {
                    break;
                }

                if current_epoch >= target_epoch {
                    break;
                }
                if let Some(current_cluster_stake) = history.get_entry(current_epoch) {
                    prev_epoch = current_epoch;
                    prev_cluster_stake = current_cluster_stake;
                } else {
                    break;
                }
            }

            // deactivating stake should equal to all of currently remaining effective stake
            StakeActivationStatus::with_deactivating(current_effective_stake)
        } else {
            // no history or I've dropped out of history, so assume fully deactivated
            StakeActivationStatus::default()
        }
    }

    // returned tuple is (effective, activating) stake
    fn stake_and_activating<T: StakeHistoryGetEntry>(
        &self,
        target_epoch: Epoch,
        history: &T,
        new_rate_activation_epoch: Option<Epoch>,
    ) -> (u64, u64) {
        let delegated_stake = self.stake;

        if self.is_bootstrap() {
            // fully effective immediately
            (delegated_stake, 0)
        } else if self.activation_epoch == self.deactivation_epoch {
            // activated but instantly deactivated; no stake at all regardless of target_epoch
            // this must be after the bootstrap check and before all-is-activating check
            (0, 0)
        } else if target_epoch == self.activation_epoch {
            // all is activating
            (0, delegated_stake)
        } else if target_epoch < self.activation_epoch {
            // not yet enabled
            (0, 0)
        } else if let Some((history, mut prev_epoch, mut prev_cluster_stake)) = history
            .get_entry(self.activation_epoch)
            .map(|cluster_stake_at_activation_epoch| {
                (
                    history,
                    self.activation_epoch,
                    cluster_stake_at_activation_epoch,
                )
            })
        {
            // target_epoch > self.activation_epoch

            // loop from my activation epoch until the target epoch summing up my entitlement
            // current effective stake is updated using its previous epoch's cluster stake
            let mut current_epoch;
            let mut current_effective_stake = 0;
            loop {
                current_epoch = prev_epoch + 1;
                // if there is no activating stake at prev epoch, we should have been
                // fully effective at this moment
                if prev_cluster_stake.activating == 0 {
                    break;
                }

                // how much of the growth in stake this account is
                //  entitled to take
                let remaining_activating_stake = delegated_stake - current_effective_stake;
                let weight =
                    remaining_activating_stake as f64 / prev_cluster_stake.activating as f64;
                let warmup_cooldown_rate =
                    warmup_cooldown_rate(current_epoch, new_rate_activation_epoch);

                // portion of newly effective cluster stake I'm entitled to at current epoch
                let newly_effective_cluster_stake =
                    prev_cluster_stake.effective as f64 * warmup_cooldown_rate;
                let newly_effective_stake =
                    ((weight * newly_effective_cluster_stake) as u64).max(1);

                current_effective_stake += newly_effective_stake;
                if current_effective_stake >= delegated_stake {
                    current_effective_stake = delegated_stake;
                    break;
                }

                if current_epoch >= target_epoch || current_epoch >= self.deactivation_epoch {
                    break;
                }
                if let Some(current_cluster_stake) = history.get_entry(current_epoch) {
                    prev_epoch = current_epoch;
                    prev_cluster_stake = current_cluster_stake;
                } else {
                    break;
                }
            }

            (
                current_effective_stake,
                delegated_stake - current_effective_stake,
            )
        } else {
            // no history or I've dropped out of history, so assume fully effective
            (delegated_stake, 0)
        }
    }
}

#[cfg_attr(feature = "frozen-abi", derive(solana_frozen_abi_macro::AbiExample))]
#[cfg_attr(
    feature = "borsh",
    derive(BorshSerialize, BorshDeserialize, BorshSchema),
    borsh(crate = "borsh")
)]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Deserialize, serde_derive::Serialize)
)]
#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct Stake {
    pub delegation: Delegation,
    /// credits observed is credits from vote account state when delegated or redeemed
    pub credits_observed: u64,
}

impl Stake {
    pub fn stake<T: StakeHistoryGetEntry>(
        &self,
        epoch: Epoch,
        history: &T,
        new_rate_activation_epoch: Option<Epoch>,
    ) -> u64 {
        self.delegation
            .stake(epoch, history, new_rate_activation_epoch)
    }

    pub fn split(
        &mut self,
        remaining_stake_delta: u64,
        split_stake_amount: u64,
    ) -> Result<Self, StakeError> {
        if remaining_stake_delta > self.delegation.stake {
            return Err(StakeError::InsufficientStake);
        }
        self.delegation.stake -= remaining_stake_delta;
        let new = Self {
            delegation: Delegation {
                stake: split_stake_amount,
                ..self.delegation
            },
            ..*self
        };
        Ok(new)
    }

    pub fn deactivate(&mut self, epoch: Epoch) -> Result<(), StakeError> {
        if self.delegation.deactivation_epoch != u64::MAX {
            Err(StakeError::AlreadyDeactivated)
        } else {
            self.delegation.deactivation_epoch = epoch;
            Ok(())
        }
    }
}

#[cfg(all(feature = "borsh", feature = "bincode"))]
#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::stake_history::StakeHistory,
        assert_matches::assert_matches,
        bincode::serialize,
        solana_account::{state_traits::StateMut, AccountSharedData, ReadableAccount},
        solana_borsh::v1::try_from_slice_unchecked,
        solana_pubkey::Pubkey,
        test_case::test_case,
    };

    fn from<T: ReadableAccount + StateMut<StakeStateV2>>(account: &T) -> Option<StakeStateV2> {
        account.state().ok()
    }

    fn stake_from<T: ReadableAccount + StateMut<StakeStateV2>>(account: &T) -> Option<Stake> {
        from(account).and_then(|state: StakeStateV2| state.stake())
    }

    fn new_stake_history_entry<'a, I>(
        epoch: Epoch,
        stakes: I,
        history: &StakeHistory,
        new_rate_activation_epoch: Option<Epoch>,
    ) -> StakeHistoryEntry
    where
        I: Iterator<Item = &'a Delegation>,
    {
        stakes.fold(StakeHistoryEntry::default(), |sum, stake| {
            sum + stake.stake_activating_and_deactivating(epoch, history, new_rate_activation_epoch)
        })
    }

    fn create_stake_history_from_delegations(
        bootstrap: Option<u64>,
        epochs: std::ops::Range<Epoch>,
        delegations: &[Delegation],
        new_rate_activation_epoch: Option<Epoch>,
    ) -> StakeHistory {
        let mut stake_history = StakeHistory::default();

        let bootstrap_delegation = if let Some(bootstrap) = bootstrap {
            vec![Delegation {
                activation_epoch: u64::MAX,
                stake: bootstrap,
                ..Delegation::default()
            }]
        } else {
            vec![]
        };

        for epoch in epochs {
            let entry = new_stake_history_entry(
                epoch,
                delegations.iter().chain(bootstrap_delegation.iter()),
                &stake_history,
                new_rate_activation_epoch,
            );
            stake_history.add(epoch, entry);
        }

        stake_history
    }

    #[test]
    fn test_authorized_authorize() {
        let staker = Pubkey::new_unique();
        let mut authorized = Authorized::auto(&staker);
        let mut signers = HashSet::new();
        assert_eq!(
            authorized.authorize(&signers, &staker, StakeAuthorize::Staker, None),
            Err(InstructionError::MissingRequiredSignature)
        );
        signers.insert(staker);
        assert_eq!(
            authorized.authorize(&signers, &staker, StakeAuthorize::Staker, None),
            Ok(())
        );
    }

    #[test]
    fn test_authorized_authorize_with_custodian() {
        let staker = Pubkey::new_unique();
        let custodian = Pubkey::new_unique();
        let invalid_custodian = Pubkey::new_unique();
        let mut authorized = Authorized::auto(&staker);
        let mut signers = HashSet::new();
        signers.insert(staker);

        let lockup = Lockup {
            epoch: 1,
            unix_timestamp: 1,
            custodian,
        };
        let clock = Clock {
            epoch: 0,
            unix_timestamp: 0,
            ..Clock::default()
        };

        // No lockup, no custodian
        assert_eq!(
            authorized.authorize(
                &signers,
                &staker,
                StakeAuthorize::Withdrawer,
                Some((&Lockup::default(), &clock, None))
            ),
            Ok(())
        );

        // No lockup, invalid custodian not a signer
        assert_eq!(
            authorized.authorize(
                &signers,
                &staker,
                StakeAuthorize::Withdrawer,
                Some((&Lockup::default(), &clock, Some(&invalid_custodian)))
            ),
            Ok(()) // <== invalid custodian doesn't matter, there's no lockup
        );

        // Lockup active, invalid custodian not a signer
        assert_eq!(
            authorized.authorize(
                &signers,
                &staker,
                StakeAuthorize::Withdrawer,
                Some((&lockup, &clock, Some(&invalid_custodian)))
            ),
            Err(StakeError::CustodianSignatureMissing.into()),
        );

        signers.insert(invalid_custodian);

        // No lockup, invalid custodian is a signer
        assert_eq!(
            authorized.authorize(
                &signers,
                &staker,
                StakeAuthorize::Withdrawer,
                Some((&Lockup::default(), &clock, Some(&invalid_custodian)))
            ),
            Ok(()) // <== invalid custodian doesn't matter, there's no lockup
        );

        // Lockup active, invalid custodian is a signer
        signers.insert(invalid_custodian);
        assert_eq!(
            authorized.authorize(
                &signers,
                &staker,
                StakeAuthorize::Withdrawer,
                Some((&lockup, &clock, Some(&invalid_custodian)))
            ),
            Err(StakeError::LockupInForce.into()), // <== invalid custodian rejected
        );

        signers.remove(&invalid_custodian);

        // Lockup active, no custodian
        assert_eq!(
            authorized.authorize(
                &signers,
                &staker,
                StakeAuthorize::Withdrawer,
                Some((&lockup, &clock, None))
            ),
            Err(StakeError::CustodianMissing.into()),
        );

        // Lockup active, custodian not a signer
        assert_eq!(
            authorized.authorize(
                &signers,
                &staker,
                StakeAuthorize::Withdrawer,
                Some((&lockup, &clock, Some(&custodian)))
            ),
            Err(StakeError::CustodianSignatureMissing.into()),
        );

        // Lockup active, custodian is a signer
        signers.insert(custodian);
        assert_eq!(
            authorized.authorize(
                &signers,
                &staker,
                StakeAuthorize::Withdrawer,
                Some((&lockup, &clock, Some(&custodian)))
            ),
            Ok(())
        );
    }

    #[test]
    fn test_stake_state_stake_from_fail() {
        let mut stake_account =
            AccountSharedData::new(0, StakeStateV2::size_of(), &crate::program::id());

        stake_account
            .set_state(&StakeStateV2::default())
            .expect("set_state");

        assert_eq!(stake_from(&stake_account), None);
    }

    #[test]
    fn test_stake_is_bootstrap() {
        assert!(Delegation {
            activation_epoch: u64::MAX,
            ..Delegation::default()
        }
        .is_bootstrap());
        assert!(!Delegation {
            activation_epoch: 0,
            ..Delegation::default()
        }
        .is_bootstrap());
    }

    #[test]
    fn test_stake_activating_and_deactivating() {
        let stake = Delegation {
            stake: 1_000,
            activation_epoch: 0, // activating at zero
            deactivation_epoch: 5,
            ..Delegation::default()
        };

        // save this off so stake.config.warmup_rate changes don't break this test
        let increment = (1_000_f64 * warmup_cooldown_rate(0, None)) as u64;

        let mut stake_history = StakeHistory::default();
        // assert that this stake follows step function if there's no history
        assert_eq!(
            stake.stake_activating_and_deactivating(stake.activation_epoch, &stake_history, None),
            StakeActivationStatus::with_effective_and_activating(0, stake.stake),
        );
        for epoch in stake.activation_epoch + 1..stake.deactivation_epoch {
            assert_eq!(
                stake.stake_activating_and_deactivating(epoch, &stake_history, None),
                StakeActivationStatus::with_effective(stake.stake),
            );
        }
        // assert that this stake is full deactivating
        assert_eq!(
            stake.stake_activating_and_deactivating(stake.deactivation_epoch, &stake_history, None),
            StakeActivationStatus::with_deactivating(stake.stake),
        );
        // assert that this stake is fully deactivated if there's no history
        assert_eq!(
            stake.stake_activating_and_deactivating(
                stake.deactivation_epoch + 1,
                &stake_history,
                None
            ),
            StakeActivationStatus::default(),
        );

        stake_history.add(
            0u64, // entry for zero doesn't have my activating amount
            StakeHistoryEntry {
                effective: 1_000,
                ..StakeHistoryEntry::default()
            },
        );
        // assert that this stake is broken, because above setup is broken
        assert_eq!(
            stake.stake_activating_and_deactivating(1, &stake_history, None),
            StakeActivationStatus::with_effective_and_activating(0, stake.stake),
        );

        stake_history.add(
            0u64, // entry for zero has my activating amount
            StakeHistoryEntry {
                effective: 1_000,
                activating: 1_000,
                ..StakeHistoryEntry::default()
            },
            // no entry for 1, so this stake gets shorted
        );
        // assert that this stake is broken, because above setup is broken
        assert_eq!(
            stake.stake_activating_and_deactivating(2, &stake_history, None),
            StakeActivationStatus::with_effective_and_activating(
                increment,
                stake.stake - increment
            ),
        );

        // start over, test deactivation edge cases
        let mut stake_history = StakeHistory::default();

        stake_history.add(
            stake.deactivation_epoch, // entry for zero doesn't have my de-activating amount
            StakeHistoryEntry {
                effective: 1_000,
                ..StakeHistoryEntry::default()
            },
        );
        // assert that this stake is broken, because above setup is broken
        assert_eq!(
            stake.stake_activating_and_deactivating(
                stake.deactivation_epoch + 1,
                &stake_history,
                None,
            ),
            StakeActivationStatus::with_deactivating(stake.stake),
        );

        // put in my initial deactivating amount, but don't put in an entry for next
        stake_history.add(
            stake.deactivation_epoch, // entry for zero has my de-activating amount
            StakeHistoryEntry {
                effective: 1_000,
                deactivating: 1_000,
                ..StakeHistoryEntry::default()
            },
        );
        // assert that this stake is broken, because above setup is broken
        assert_eq!(
            stake.stake_activating_and_deactivating(
                stake.deactivation_epoch + 2,
                &stake_history,
                None,
            ),
            // hung, should be lower
            StakeActivationStatus::with_deactivating(stake.stake - increment),
        );
    }

    mod same_epoch_activation_then_deactivation {
        use super::*;

        enum OldDeactivationBehavior {
            Stuck,
            Slow,
        }

        fn do_test(
            old_behavior: OldDeactivationBehavior,
            expected_stakes: &[StakeActivationStatus],
        ) {
            let cluster_stake = 1_000;
            let activating_stake = 10_000;
            let some_stake = 700;
            let some_epoch = 0;

            let stake = Delegation {
                stake: some_stake,
                activation_epoch: some_epoch,
                deactivation_epoch: some_epoch,
                ..Delegation::default()
            };

            let mut stake_history = StakeHistory::default();
            let cluster_deactivation_at_stake_modified_epoch = match old_behavior {
                OldDeactivationBehavior::Stuck => 0,
                OldDeactivationBehavior::Slow => 1000,
            };

            let stake_history_entries = vec![
                (
                    cluster_stake,
                    activating_stake,
                    cluster_deactivation_at_stake_modified_epoch,
                ),
                (cluster_stake, activating_stake, 1000),
                (cluster_stake, activating_stake, 1000),
                (cluster_stake, activating_stake, 100),
                (cluster_stake, activating_stake, 100),
                (cluster_stake, activating_stake, 100),
                (cluster_stake, activating_stake, 100),
            ];

            for (epoch, (effective, activating, deactivating)) in
                stake_history_entries.into_iter().enumerate()
            {
                stake_history.add(
                    epoch as Epoch,
                    StakeHistoryEntry {
                        effective,
                        activating,
                        deactivating,
                    },
                );
            }

            assert_eq!(
                expected_stakes,
                (0..expected_stakes.len())
                    .map(|epoch| stake.stake_activating_and_deactivating(
                        epoch as u64,
                        &stake_history,
                        None,
                    ))
                    .collect::<Vec<_>>()
            );
        }

        #[test]
        fn test_new_behavior_previously_slow() {
            // any stake accounts activated and deactivated at the same epoch
            // shouldn't been activated (then deactivated) at all!

            do_test(
                OldDeactivationBehavior::Slow,
                &[
                    StakeActivationStatus::default(),
                    StakeActivationStatus::default(),
                    StakeActivationStatus::default(),
                    StakeActivationStatus::default(),
                    StakeActivationStatus::default(),
                    StakeActivationStatus::default(),
                    StakeActivationStatus::default(),
                ],
            );
        }

        #[test]
        fn test_new_behavior_previously_stuck() {
            // any stake accounts activated and deactivated at the same epoch
            // shouldn't been activated (then deactivated) at all!

            do_test(
                OldDeactivationBehavior::Stuck,
                &[
                    StakeActivationStatus::default(),
                    StakeActivationStatus::default(),
                    StakeActivationStatus::default(),
                    StakeActivationStatus::default(),
                    StakeActivationStatus::default(),
                    StakeActivationStatus::default(),
                    StakeActivationStatus::default(),
                ],
            );
        }
    }

    #[test]
    fn test_inflation_and_slashing_with_activating_and_deactivating_stake() {
        // some really boring delegation and stake_history setup
        let (delegated_stake, mut stake, stake_history) = {
            let cluster_stake = 1_000;
            let delegated_stake = 700;

            let stake = Delegation {
                stake: delegated_stake,
                activation_epoch: 0,
                deactivation_epoch: 4,
                ..Delegation::default()
            };

            let mut stake_history = StakeHistory::default();
            stake_history.add(
                0,
                StakeHistoryEntry {
                    effective: cluster_stake,
                    activating: delegated_stake,
                    ..StakeHistoryEntry::default()
                },
            );
            let newly_effective_at_epoch1 = (cluster_stake as f64 * 0.25) as u64;
            assert_eq!(newly_effective_at_epoch1, 250);
            stake_history.add(
                1,
                StakeHistoryEntry {
                    effective: cluster_stake + newly_effective_at_epoch1,
                    activating: delegated_stake - newly_effective_at_epoch1,
                    ..StakeHistoryEntry::default()
                },
            );
            let newly_effective_at_epoch2 =
                ((cluster_stake + newly_effective_at_epoch1) as f64 * 0.25) as u64;
            assert_eq!(newly_effective_at_epoch2, 312);
            stake_history.add(
                2,
                StakeHistoryEntry {
                    effective: cluster_stake
                        + newly_effective_at_epoch1
                        + newly_effective_at_epoch2,
                    activating: delegated_stake
                        - newly_effective_at_epoch1
                        - newly_effective_at_epoch2,
                    ..StakeHistoryEntry::default()
                },
            );
            stake_history.add(
                3,
                StakeHistoryEntry {
                    effective: cluster_stake + delegated_stake,
                    ..StakeHistoryEntry::default()
                },
            );
            stake_history.add(
                4,
                StakeHistoryEntry {
                    effective: cluster_stake + delegated_stake,
                    deactivating: delegated_stake,
                    ..StakeHistoryEntry::default()
                },
            );
            let newly_not_effective_stake_at_epoch5 =
                ((cluster_stake + delegated_stake) as f64 * 0.25) as u64;
            assert_eq!(newly_not_effective_stake_at_epoch5, 425);
            stake_history.add(
                5,
                StakeHistoryEntry {
                    effective: cluster_stake + delegated_stake
                        - newly_not_effective_stake_at_epoch5,
                    deactivating: delegated_stake - newly_not_effective_stake_at_epoch5,
                    ..StakeHistoryEntry::default()
                },
            );

            (delegated_stake, stake, stake_history)
        };

        // helper closures
        let calculate_each_staking_status = |stake: &Delegation, epoch_count: usize| -> Vec<_> {
            (0..epoch_count)
                .map(|epoch| {
                    stake.stake_activating_and_deactivating(epoch as u64, &stake_history, None)
                })
                .collect::<Vec<_>>()
        };
        let adjust_staking_status = |rate: f64, status: &[StakeActivationStatus]| {
            status
                .iter()
                .map(|entry| StakeActivationStatus {
                    effective: (entry.effective as f64 * rate) as u64,
                    activating: (entry.activating as f64 * rate) as u64,
                    deactivating: (entry.deactivating as f64 * rate) as u64,
                })
                .collect::<Vec<_>>()
        };

        let expected_staking_status_transition = vec![
            StakeActivationStatus::with_effective_and_activating(0, 700),
            StakeActivationStatus::with_effective_and_activating(250, 450),
            StakeActivationStatus::with_effective_and_activating(562, 138),
            StakeActivationStatus::with_effective(700),
            StakeActivationStatus::with_deactivating(700),
            StakeActivationStatus::with_deactivating(275),
            StakeActivationStatus::default(),
        ];
        let expected_staking_status_transition_base = vec![
            StakeActivationStatus::with_effective_and_activating(0, 700),
            StakeActivationStatus::with_effective_and_activating(250, 450),
            StakeActivationStatus::with_effective_and_activating(562, 138 + 1), // +1 is needed for rounding
            StakeActivationStatus::with_effective(700),
            StakeActivationStatus::with_deactivating(700),
            StakeActivationStatus::with_deactivating(275 + 1), // +1 is needed for rounding
            StakeActivationStatus::default(),
        ];

        // normal stake activating and deactivating transition test, just in case
        assert_eq!(
            expected_staking_status_transition,
            calculate_each_staking_status(&stake, expected_staking_status_transition.len())
        );

        // 10% inflation rewards assuming some sizable epochs passed!
        let rate = 1.10;
        stake.stake = (delegated_stake as f64 * rate) as u64;
        let expected_staking_status_transition =
            adjust_staking_status(rate, &expected_staking_status_transition_base);

        assert_eq!(
            expected_staking_status_transition,
            calculate_each_staking_status(&stake, expected_staking_status_transition_base.len()),
        );

        // 50% slashing!!!
        let rate = 0.5;
        stake.stake = (delegated_stake as f64 * rate) as u64;
        let expected_staking_status_transition =
            adjust_staking_status(rate, &expected_staking_status_transition_base);

        assert_eq!(
            expected_staking_status_transition,
            calculate_each_staking_status(&stake, expected_staking_status_transition_base.len()),
        );
    }

    #[test]
    fn test_stop_activating_after_deactivation() {
        let stake = Delegation {
            stake: 1_000,
            activation_epoch: 0,
            deactivation_epoch: 3,
            ..Delegation::default()
        };

        let base_stake = 1_000;
        let mut stake_history = StakeHistory::default();
        let mut effective = base_stake;
        let other_activation = 100;
        let mut other_activations = vec![0];

        // Build a stake history where the test staker always consumes all of the available warm
        // up and cool down stake. However, simulate other stakers beginning to activate during
        // the test staker's deactivation.
        for epoch in 0..=stake.deactivation_epoch + 1 {
            let (activating, deactivating) = if epoch < stake.deactivation_epoch {
                (stake.stake + base_stake - effective, 0)
            } else {
                let other_activation_sum: u64 = other_activations.iter().sum();
                let deactivating = effective - base_stake - other_activation_sum;
                (other_activation, deactivating)
            };

            stake_history.add(
                epoch,
                StakeHistoryEntry {
                    effective,
                    activating,
                    deactivating,
                },
            );

            let effective_rate_limited = (effective as f64 * warmup_cooldown_rate(0, None)) as u64;
            if epoch < stake.deactivation_epoch {
                effective += effective_rate_limited.min(activating);
                other_activations.push(0);
            } else {
                effective -= effective_rate_limited.min(deactivating);
                effective += other_activation;
                other_activations.push(other_activation);
            }
        }

        for epoch in 0..=stake.deactivation_epoch + 1 {
            let history = stake_history.get(epoch).unwrap();
            let other_activations: u64 = other_activations[..=epoch as usize].iter().sum();
            let expected_stake = history.effective - base_stake - other_activations;
            let (expected_activating, expected_deactivating) = if epoch < stake.deactivation_epoch {
                (history.activating, 0)
            } else {
                (0, history.deactivating)
            };
            assert_eq!(
                stake.stake_activating_and_deactivating(epoch, &stake_history, None),
                StakeActivationStatus {
                    effective: expected_stake,
                    activating: expected_activating,
                    deactivating: expected_deactivating,
                },
            );
        }
    }

    #[test]
    fn test_stake_warmup_cooldown_sub_integer_moves() {
        let delegations = [Delegation {
            stake: 2,
            activation_epoch: 0, // activating at zero
            deactivation_epoch: 5,
            ..Delegation::default()
        }];
        // give 2 epochs of cooldown
        let epochs = 7;
        // make bootstrap stake smaller than warmup so warmup/cooldownn
        //  increment is always smaller than 1
        let bootstrap = (warmup_cooldown_rate(0, None) * 100.0 / 2.0) as u64;
        let stake_history =
            create_stake_history_from_delegations(Some(bootstrap), 0..epochs, &delegations, None);
        let mut max_stake = 0;
        let mut min_stake = 2;

        for epoch in 0..epochs {
            let stake = delegations
                .iter()
                .map(|delegation| delegation.stake(epoch, &stake_history, None))
                .sum::<u64>();
            max_stake = max_stake.max(stake);
            min_stake = min_stake.min(stake);
        }
        assert_eq!(max_stake, 2);
        assert_eq!(min_stake, 0);
    }

    #[test_case(None ; "old rate")]
    #[test_case(Some(1) ; "new rate activated in epoch 1")]
    #[test_case(Some(10) ; "new rate activated in epoch 10")]
    #[test_case(Some(30) ; "new rate activated in epoch 30")]
    #[test_case(Some(50) ; "new rate activated in epoch 50")]
    #[test_case(Some(60) ; "new rate activated in epoch 60")]
    fn test_stake_warmup_cooldown(new_rate_activation_epoch: Option<Epoch>) {
        let delegations = [
            Delegation {
                // never deactivates
                stake: 1_000,
                activation_epoch: u64::MAX,
                ..Delegation::default()
            },
            Delegation {
                stake: 1_000,
                activation_epoch: 0,
                deactivation_epoch: 9,
                ..Delegation::default()
            },
            Delegation {
                stake: 1_000,
                activation_epoch: 1,
                deactivation_epoch: 6,
                ..Delegation::default()
            },
            Delegation {
                stake: 1_000,
                activation_epoch: 2,
                deactivation_epoch: 5,
                ..Delegation::default()
            },
            Delegation {
                stake: 1_000,
                activation_epoch: 2,
                deactivation_epoch: 4,
                ..Delegation::default()
            },
            Delegation {
                stake: 1_000,
                activation_epoch: 4,
                deactivation_epoch: 4,
                ..Delegation::default()
            },
        ];
        // chosen to ensure that the last activated stake (at 4) finishes
        //  warming up and cooling down
        //  a stake takes 2.0f64.log(1.0 + STAKE_WARMUP_RATE) epochs to warm up or cool down
        //  when all alone, but the above overlap a lot
        let epochs = 60;

        let stake_history = create_stake_history_from_delegations(
            None,
            0..epochs,
            &delegations,
            new_rate_activation_epoch,
        );

        let mut prev_total_effective_stake = delegations
            .iter()
            .map(|delegation| delegation.stake(0, &stake_history, new_rate_activation_epoch))
            .sum::<u64>();

        // uncomment and add ! for fun with graphing
        // eprintln("\n{:8} {:8} {:8}", "   epoch", "   total", "   delta");
        for epoch in 1..epochs {
            let total_effective_stake = delegations
                .iter()
                .map(|delegation| {
                    delegation.stake(epoch, &stake_history, new_rate_activation_epoch)
                })
                .sum::<u64>();

            let delta = if total_effective_stake > prev_total_effective_stake {
                total_effective_stake - prev_total_effective_stake
            } else {
                prev_total_effective_stake - total_effective_stake
            };

            // uncomment and add ! for fun with graphing
            // eprint("{:8} {:8} {:8} ", epoch, total_effective_stake, delta);
            // (0..(total_effective_stake as usize / (delegations.len() * 5))).for_each(|_| eprint("#"));
            // eprintln();

            assert!(
                delta
                    <= ((prev_total_effective_stake as f64
                        * warmup_cooldown_rate(epoch, new_rate_activation_epoch))
                        as u64)
                        .max(1)
            );

            prev_total_effective_stake = total_effective_stake;
        }
    }

    #[test]
    fn test_lockup_is_expired() {
        let custodian = Pubkey::new_unique();
        let lockup = Lockup {
            epoch: 1,
            unix_timestamp: 1,
            custodian,
        };
        // neither time
        assert!(lockup.is_in_force(
            &Clock {
                epoch: 0,
                unix_timestamp: 0,
                ..Clock::default()
            },
            None
        ));
        // not timestamp
        assert!(lockup.is_in_force(
            &Clock {
                epoch: 2,
                unix_timestamp: 0,
                ..Clock::default()
            },
            None
        ));
        // not epoch
        assert!(lockup.is_in_force(
            &Clock {
                epoch: 0,
                unix_timestamp: 2,
                ..Clock::default()
            },
            None
        ));
        // both, no custodian
        assert!(!lockup.is_in_force(
            &Clock {
                epoch: 1,
                unix_timestamp: 1,
                ..Clock::default()
            },
            None
        ));
        // neither, but custodian
        assert!(!lockup.is_in_force(
            &Clock {
                epoch: 0,
                unix_timestamp: 0,
                ..Clock::default()
            },
            Some(&custodian),
        ));
    }

    fn check_borsh_deserialization(stake: StakeStateV2) {
        let serialized = serialize(&stake).unwrap();
        let deserialized = StakeStateV2::try_from_slice(&serialized).unwrap();
        assert_eq!(stake, deserialized);
    }

    fn check_borsh_serialization(stake: StakeStateV2) {
        let bincode_serialized = serialize(&stake).unwrap();
        let borsh_serialized = borsh::to_vec(&stake).unwrap();
        assert_eq!(bincode_serialized, borsh_serialized);
    }

    #[test]
    fn test_size_of() {
        assert_eq!(StakeStateV2::size_of(), std::mem::size_of::<StakeStateV2>());
    }

    #[test]
    fn bincode_vs_borsh_deserialization() {
        check_borsh_deserialization(StakeStateV2::Uninitialized);
        check_borsh_deserialization(StakeStateV2::RewardsPool);
        check_borsh_deserialization(StakeStateV2::Initialized(Meta {
            rent_exempt_reserve: u64::MAX,
            authorized: Authorized {
                staker: Pubkey::new_unique(),
                withdrawer: Pubkey::new_unique(),
            },
            lockup: Lockup::default(),
        }));
        check_borsh_deserialization(StakeStateV2::Stake(
            Meta {
                rent_exempt_reserve: 1,
                authorized: Authorized {
                    staker: Pubkey::new_unique(),
                    withdrawer: Pubkey::new_unique(),
                },
                lockup: Lockup::default(),
            },
            Stake {
                delegation: Delegation {
                    voter_pubkey: Pubkey::new_unique(),
                    stake: u64::MAX,
                    activation_epoch: Epoch::MAX,
                    deactivation_epoch: Epoch::MAX,
                    ..Delegation::default()
                },
                credits_observed: 1,
            },
            StakeFlags::empty(),
        ));
    }

    #[test]
    fn bincode_vs_borsh_serialization() {
        check_borsh_serialization(StakeStateV2::Uninitialized);
        check_borsh_serialization(StakeStateV2::RewardsPool);
        check_borsh_serialization(StakeStateV2::Initialized(Meta {
            rent_exempt_reserve: u64::MAX,
            authorized: Authorized {
                staker: Pubkey::new_unique(),
                withdrawer: Pubkey::new_unique(),
            },
            lockup: Lockup::default(),
        }));
        #[allow(deprecated)]
        check_borsh_serialization(StakeStateV2::Stake(
            Meta {
                rent_exempt_reserve: 1,
                authorized: Authorized {
                    staker: Pubkey::new_unique(),
                    withdrawer: Pubkey::new_unique(),
                },
                lockup: Lockup::default(),
            },
            Stake {
                delegation: Delegation {
                    voter_pubkey: Pubkey::new_unique(),
                    stake: u64::MAX,
                    activation_epoch: Epoch::MAX,
                    deactivation_epoch: Epoch::MAX,
                    ..Default::default()
                },
                credits_observed: 1,
            },
            StakeFlags::MUST_FULLY_ACTIVATE_BEFORE_DEACTIVATION_IS_PERMITTED,
        ));
    }

    #[test]
    fn borsh_deserialization_live_data() {
        let data = [
            1, 0, 0, 0, 128, 213, 34, 0, 0, 0, 0, 0, 133, 0, 79, 231, 141, 29, 73, 61, 232, 35,
            119, 124, 168, 12, 120, 216, 195, 29, 12, 166, 139, 28, 36, 182, 186, 154, 246, 149,
            224, 109, 52, 100, 133, 0, 79, 231, 141, 29, 73, 61, 232, 35, 119, 124, 168, 12, 120,
            216, 195, 29, 12, 166, 139, 28, 36, 182, 186, 154, 246, 149, 224, 109, 52, 100, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        // As long as we get the 4-byte enum and the first field right, then
        // we're sure the rest works out
        let deserialized = try_from_slice_unchecked::<StakeStateV2>(&data).unwrap();
        assert_matches!(
            deserialized,
            StakeStateV2::Initialized(Meta {
                rent_exempt_reserve: 2282880,
                ..
            })
        );
    }

    #[test]
    fn stake_flag_member_offset() {
        const FLAG_OFFSET: usize = 196;
        let check_flag = |flag, expected| {
            let stake = StakeStateV2::Stake(
                Meta {
                    rent_exempt_reserve: 1,
                    authorized: Authorized {
                        staker: Pubkey::new_unique(),
                        withdrawer: Pubkey::new_unique(),
                    },
                    lockup: Lockup::default(),
                },
                Stake {
                    delegation: Delegation {
                        voter_pubkey: Pubkey::new_unique(),
                        stake: u64::MAX,
                        activation_epoch: Epoch::MAX,
                        deactivation_epoch: Epoch::MAX,
                        warmup_cooldown_rate: f64::MAX,
                    },
                    credits_observed: 1,
                },
                flag,
            );

            let bincode_serialized = serialize(&stake).unwrap();
            let borsh_serialized = borsh::to_vec(&stake).unwrap();

            assert_eq!(bincode_serialized[FLAG_OFFSET], expected);
            assert_eq!(borsh_serialized[FLAG_OFFSET], expected);
        };
        #[allow(deprecated)]
        check_flag(
            StakeFlags::MUST_FULLY_ACTIVATE_BEFORE_DEACTIVATION_IS_PERMITTED,
            1,
        );
        check_flag(StakeFlags::empty(), 0);
    }

    mod deprecated {
        use super::*;

        fn check_borsh_deserialization(stake: StakeState) {
            let serialized = serialize(&stake).unwrap();
            let deserialized = StakeState::try_from_slice(&serialized).unwrap();
            assert_eq!(stake, deserialized);
        }

        fn check_borsh_serialization(stake: StakeState) {
            let bincode_serialized = serialize(&stake).unwrap();
            let borsh_serialized = borsh::to_vec(&stake).unwrap();
            assert_eq!(bincode_serialized, borsh_serialized);
        }

        #[test]
        fn test_size_of() {
            assert_eq!(StakeState::size_of(), std::mem::size_of::<StakeState>());
        }

        #[test]
        fn bincode_vs_borsh_deserialization() {
            check_borsh_deserialization(StakeState::Uninitialized);
            check_borsh_deserialization(StakeState::RewardsPool);
            check_borsh_deserialization(StakeState::Initialized(Meta {
                rent_exempt_reserve: u64::MAX,
                authorized: Authorized {
                    staker: Pubkey::new_unique(),
                    withdrawer: Pubkey::new_unique(),
                },
                lockup: Lockup::default(),
            }));
            check_borsh_deserialization(StakeState::Stake(
                Meta {
                    rent_exempt_reserve: 1,
                    authorized: Authorized {
                        staker: Pubkey::new_unique(),
                        withdrawer: Pubkey::new_unique(),
                    },
                    lockup: Lockup::default(),
                },
                Stake {
                    delegation: Delegation {
                        voter_pubkey: Pubkey::new_unique(),
                        stake: u64::MAX,
                        activation_epoch: Epoch::MAX,
                        deactivation_epoch: Epoch::MAX,
                        warmup_cooldown_rate: f64::MAX,
                    },
                    credits_observed: 1,
                },
            ));
        }

        #[test]
        fn bincode_vs_borsh_serialization() {
            check_borsh_serialization(StakeState::Uninitialized);
            check_borsh_serialization(StakeState::RewardsPool);
            check_borsh_serialization(StakeState::Initialized(Meta {
                rent_exempt_reserve: u64::MAX,
                authorized: Authorized {
                    staker: Pubkey::new_unique(),
                    withdrawer: Pubkey::new_unique(),
                },
                lockup: Lockup::default(),
            }));
            check_borsh_serialization(StakeState::Stake(
                Meta {
                    rent_exempt_reserve: 1,
                    authorized: Authorized {
                        staker: Pubkey::new_unique(),
                        withdrawer: Pubkey::new_unique(),
                    },
                    lockup: Lockup::default(),
                },
                Stake {
                    delegation: Delegation {
                        voter_pubkey: Pubkey::new_unique(),
                        stake: u64::MAX,
                        activation_epoch: Epoch::MAX,
                        deactivation_epoch: Epoch::MAX,
                        warmup_cooldown_rate: f64::MAX,
                    },
                    credits_observed: 1,
                },
            ));
        }

        #[test]
        fn borsh_deserialization_live_data() {
            let data = [
                1, 0, 0, 0, 128, 213, 34, 0, 0, 0, 0, 0, 133, 0, 79, 231, 141, 29, 73, 61, 232, 35,
                119, 124, 168, 12, 120, 216, 195, 29, 12, 166, 139, 28, 36, 182, 186, 154, 246,
                149, 224, 109, 52, 100, 133, 0, 79, 231, 141, 29, 73, 61, 232, 35, 119, 124, 168,
                12, 120, 216, 195, 29, 12, 166, 139, 28, 36, 182, 186, 154, 246, 149, 224, 109, 52,
                100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ];
            // As long as we get the 4-byte enum and the first field right, then
            // we're sure the rest works out
            let deserialized = try_from_slice_unchecked::<StakeState>(&data).unwrap();
            assert_matches!(
                deserialized,
                StakeState::Initialized(Meta {
                    rent_exempt_reserve: 2282880,
                    ..
                })
            );
        }
    }
}
