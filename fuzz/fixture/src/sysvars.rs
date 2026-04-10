//! Runtime sysvars.
#![allow(deprecated)]
use {
    super::proto::{
        Clock as ProtoClock, EpochRewards as ProtoEpochRewards,
        EpochSchedule as ProtoEpochSchedule, RecentBlockhashEntry as ProtoRecentBlockhashEntry,
        RecentBlockhashes as ProtoRecentBlockhashes, Rent as ProtoRent,
        SlotHashEntry as ProtoSlotHashEntry, SlotHashes as ProtoSlotHashes,
        StakeHistory as ProtoStakeHistory, StakeHistoryEntry as ProtoStakeHistoryEntry,
        SysvarContext as ProtoSysvars,
    },
    solana_clock::Clock,
    solana_epoch_rewards::EpochRewards,
    solana_epoch_schedule::EpochSchedule,
    solana_hash::Hash,
    solana_keccak_hasher::Hasher,
    solana_rent::Rent,
    solana_slot_hashes::{SlotHash, SlotHashes},
    solana_stake_interface::stake_history::{StakeHistory, StakeHistoryEntry},
    solana_sysvar::recent_blockhashes::RecentBlockhashes,
};

/// A fixture of runtime sysvars.
#[derive(Debug, Default, PartialEq)]
pub struct Sysvars {
    /// `Clock` sysvar.
    pub clock: Clock,
    /// `EpochRewards` sysvar.
    pub epoch_rewards: EpochRewards,
    /// `EpochSchedule` sysvar.
    pub epoch_schedule: EpochSchedule,
    /// `Rent` sysvar.
    pub rent: Rent,
    /// `SlotHashes` sysvar.
    pub slot_hashes: SlotHashes,
    /// `StakeHistory` sysvar.
    pub stake_history: StakeHistory,
    /// `RecentBlockhashes` sysvar.
    pub recent_blockhashes: RecentBlockhashes,
}

impl Clone for Sysvars {
    fn clone(&self) -> Self {
        Self {
            clock: self.clock.clone(),
            epoch_rewards: self.epoch_rewards.clone(),
            epoch_schedule: self.epoch_schedule.clone(),
            rent: self.rent.clone(),
            slot_hashes: SlotHashes::new(self.slot_hashes.slot_hashes()),
            stake_history: self.stake_history.clone(),
            recent_blockhashes: self.recent_blockhashes.clone(),
        }
    }
}

// Clock sysvar.
impl From<ProtoClock> for Clock {
    fn from(value: ProtoClock) -> Self {
        Self {
            slot: value.slot,
            epoch_start_timestamp: value.epoch_start_timestamp,
            epoch: value.epoch,
            leader_schedule_epoch: value.leader_schedule_epoch,
            unix_timestamp: value.unix_timestamp,
        }
    }
}
impl From<Clock> for ProtoClock {
    fn from(value: Clock) -> Self {
        Self {
            slot: value.slot,
            epoch_start_timestamp: value.epoch_start_timestamp,
            epoch: value.epoch,
            leader_schedule_epoch: value.leader_schedule_epoch,
            unix_timestamp: value.unix_timestamp,
        }
    }
}

// Epoch rewards sysvar.
impl From<ProtoEpochRewards> for EpochRewards {
    fn from(value: ProtoEpochRewards) -> Self {
        let parent_blockhash_bytes: [u8; 32] = value
            .parent_blockhash
            .try_into()
            .expect("Invalid bytes for parent blockhash");
        let parent_blockhash = Hash::new_from_array(parent_blockhash_bytes);

        let total_points_bytes: [u8; 16] = value
            .total_points
            .try_into()
            .expect("Invalid bytes for total points");
        let total_points = u128::from_le_bytes(total_points_bytes);

        Self {
            distribution_starting_block_height: value.distribution_starting_block_height,
            num_partitions: value.num_partitions,
            parent_blockhash,
            total_points,
            total_rewards: value.total_rewards,
            distributed_rewards: value.distributed_rewards,
            active: value.active,
        }
    }
}
impl From<EpochRewards> for ProtoEpochRewards {
    fn from(value: EpochRewards) -> Self {
        Self {
            distribution_starting_block_height: value.distribution_starting_block_height,
            num_partitions: value.num_partitions,
            parent_blockhash: value.parent_blockhash.to_bytes().to_vec(),
            total_points: value.total_points.to_le_bytes().to_vec(),
            total_rewards: value.total_rewards,
            distributed_rewards: value.distributed_rewards,
            active: value.active,
        }
    }
}

// Epoch schedule sysvar.
impl From<ProtoEpochSchedule> for EpochSchedule {
    fn from(value: ProtoEpochSchedule) -> Self {
        Self {
            slots_per_epoch: value.slots_per_epoch,
            leader_schedule_slot_offset: value.leader_schedule_slot_offset,
            warmup: value.warmup,
            first_normal_epoch: value.first_normal_epoch,
            first_normal_slot: value.first_normal_slot,
        }
    }
}
impl From<EpochSchedule> for ProtoEpochSchedule {
    fn from(value: EpochSchedule) -> Self {
        Self {
            slots_per_epoch: value.slots_per_epoch,
            leader_schedule_slot_offset: value.leader_schedule_slot_offset,
            warmup: value.warmup,
            first_normal_epoch: value.first_normal_epoch,
            first_normal_slot: value.first_normal_slot,
        }
    }
}

// Rent sysvar.
impl From<ProtoRent> for Rent {
    fn from(value: ProtoRent) -> Self {
        let burn_percent =
            u8::try_from(value.burn_percent).expect("Invalid integer for burn percent");
        Self {
            lamports_per_byte_year: value.lamports_per_byte_year,
            exemption_threshold: value.exemption_threshold,
            burn_percent,
        }
    }
}
impl From<Rent> for ProtoRent {
    fn from(value: Rent) -> Self {
        Self {
            lamports_per_byte_year: value.lamports_per_byte_year,
            exemption_threshold: value.exemption_threshold,
            burn_percent: value.burn_percent.into(),
        }
    }
}

// Slot hashes sysvar.
impl From<ProtoSlotHashes> for SlotHashes {
    fn from(value: ProtoSlotHashes) -> Self {
        let slot_hashes: Vec<SlotHash> = value
            .slot_hashes
            .into_iter()
            .map(
                |ProtoSlotHashEntry {
                     slot,
                     hash: hash_bytes,
                 }| {
                    let hash_bytes: [u8; 32] =
                        hash_bytes.try_into().expect("Invalid bytes for slot hash");
                    let hash = Hash::new_from_array(hash_bytes);
                    (slot, hash)
                },
            )
            .collect();
        Self::new(&slot_hashes)
    }
}
impl From<SlotHashes> for ProtoSlotHashes {
    fn from(value: SlotHashes) -> Self {
        let slot_hashes = value
            .iter()
            .map(|(slot, hash)| ProtoSlotHashEntry {
                slot: *slot,
                hash: hash.to_bytes().to_vec(),
            })
            .collect();
        Self { slot_hashes }
    }
}

// Stake history sysvar.
impl From<ProtoStakeHistory> for StakeHistory {
    fn from(value: ProtoStakeHistory) -> Self {
        let mut stake_history = StakeHistory::default();
        for entry in value.stake_history.into_iter() {
            stake_history.add(
                entry.epoch,
                StakeHistoryEntry {
                    effective: entry.effective,
                    activating: entry.activating,
                    deactivating: entry.deactivating,
                },
            );
        }
        stake_history
    }
}
impl From<StakeHistory> for ProtoStakeHistory {
    fn from(value: StakeHistory) -> Self {
        let stake_history = value
            .iter()
            .map(|(epoch, entry)| ProtoStakeHistoryEntry {
                epoch: *epoch,
                effective: entry.effective,
                activating: entry.activating,
                deactivating: entry.deactivating,
            })
            .collect();
        Self { stake_history }
    }
}

impl From<ProtoRecentBlockhashes> for RecentBlockhashes {
    fn from(value: ProtoRecentBlockhashes) -> Self {
        let hashes: Vec<Hash> = value
            .entries
            .iter()
            .map(|entry| {
                let hash_bytes: [u8; 32] = entry
                    .blockhash
                    .clone()
                    .try_into()
                    .expect("Invalid bytes for blockhash");
                Hash::new_from_array(hash_bytes)
            })
            .collect();

        let iter_items =
            value
                .entries
                .iter()
                .enumerate()
                .zip(hashes.iter())
                .map(|((idx, entry), hash)| {
                    solana_sysvar::recent_blockhashes::IterItem(
                        idx as u64,
                        hash,
                        entry.lamports_per_signature,
                    )
                });

        RecentBlockhashes::from_iter(iter_items)
    }
}

impl From<RecentBlockhashes> for ProtoRecentBlockhashes {
    fn from(value: RecentBlockhashes) -> Self {
        let entries = value
            .iter()
            .map(|entry| ProtoRecentBlockhashEntry {
                blockhash: entry.blockhash.to_bytes().to_vec(),
                lamports_per_signature: entry.fee_calculator.lamports_per_signature,
            })
            .collect();
        Self { entries }
    }
}

// Sysvars.
impl From<ProtoSysvars> for Sysvars {
    fn from(value: ProtoSysvars) -> Self {
        Self {
            clock: value.clock.map(Into::into).unwrap_or_default(),
            epoch_rewards: value.epoch_rewards.map(Into::into).unwrap_or_default(),
            epoch_schedule: value.epoch_schedule.map(Into::into).unwrap_or_default(),
            rent: value.rent.map(Into::into).unwrap_or_default(),
            slot_hashes: value.slot_hashes.map(Into::into).unwrap_or_default(),
            stake_history: value.stake_history.map(Into::into).unwrap_or_default(),
            recent_blockhashes: value.recent_blockhashes.map(Into::into).unwrap_or_default(),
        }
    }
}
impl From<Sysvars> for ProtoSysvars {
    fn from(value: Sysvars) -> Self {
        Self {
            clock: Some(value.clock.into()),
            epoch_rewards: Some(value.epoch_rewards.into()),
            epoch_schedule: Some(value.epoch_schedule.into()),
            rent: Some(value.rent.into()),
            slot_hashes: Some(value.slot_hashes.into()),
            stake_history: Some(value.stake_history.into()),
            recent_blockhashes: Some(value.recent_blockhashes.into()),
        }
    }
}

pub(crate) fn hash_proto_sysvars(hasher: &mut Hasher, sysvars: &ProtoSysvars) {
    // Clock
    if let Some(clock) = &sysvars.clock {
        hasher.hash(&clock.slot.to_le_bytes());
        hasher.hash(&clock.epoch_start_timestamp.to_le_bytes());
        hasher.hash(&clock.epoch.to_le_bytes());
        hasher.hash(&clock.leader_schedule_epoch.to_le_bytes());
        hasher.hash(&clock.unix_timestamp.to_le_bytes());
    }
    // EpochRewards
    if let Some(epoch_rewards) = &sysvars.epoch_rewards {
        hasher.hash(
            &epoch_rewards
                .distribution_starting_block_height
                .to_le_bytes(),
        );
        hasher.hash(&epoch_rewards.num_partitions.to_le_bytes());
        hasher.hash(&epoch_rewards.parent_blockhash);
        hasher.hash(&epoch_rewards.total_points);
        hasher.hash(&epoch_rewards.total_rewards.to_le_bytes());
        hasher.hash(&epoch_rewards.distributed_rewards.to_le_bytes());
        hasher.hash(&[epoch_rewards.active as u8]);
    }
    // EpochSchedule
    if let Some(epoch_schedule) = &sysvars.epoch_schedule {
        hasher.hash(&epoch_schedule.slots_per_epoch.to_le_bytes());
        hasher.hash(&epoch_schedule.leader_schedule_slot_offset.to_le_bytes());
        hasher.hash(&[epoch_schedule.warmup as u8]);
        hasher.hash(&epoch_schedule.first_normal_epoch.to_le_bytes());
        hasher.hash(&epoch_schedule.first_normal_slot.to_le_bytes());
    }
    // Rent
    if let Some(rent) = &sysvars.rent {
        hasher.hash(&rent.lamports_per_byte_year.to_le_bytes());
        hasher.hash(&rent.exemption_threshold.to_le_bytes());
        hasher.hash(&rent.burn_percent.to_le_bytes());
    }
    // SlotHashes
    if let Some(slot_hashes) = &sysvars.slot_hashes {
        for entry in &slot_hashes.slot_hashes {
            hasher.hash(&entry.slot.to_le_bytes());
            hasher.hash(&entry.hash);
        }
    }
    // StakeHistory
    if let Some(stake_history) = &sysvars.stake_history {
        for entry in &stake_history.stake_history {
            hasher.hash(&entry.epoch.to_le_bytes());
            hasher.hash(&entry.effective.to_le_bytes());
            hasher.hash(&entry.activating.to_le_bytes());
            hasher.hash(&entry.deactivating.to_le_bytes());
        }
    }
    // RecentBlockhashes
    if let Some(recent_blockhashes) = &sysvars.recent_blockhashes {
        for entry in &recent_blockhashes.entries {
            hasher.hash(&entry.blockhash);
            hasher.hash(&entry.lamports_per_signature.to_le_bytes());
        }
    }
}
