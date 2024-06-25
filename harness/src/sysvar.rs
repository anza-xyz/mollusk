//! Module for working with Solana sysvars.

use {
    solana_program_runtime::sysvar_cache::SysvarCache,
    solana_sdk::{
        clock::{Clock, Slot},
        epoch_rewards::EpochRewards,
        epoch_schedule::EpochSchedule,
        hash::Hash,
        rent::Rent,
        slot_hashes::SlotHashes,
        stake_history::StakeHistory,
        sysvar::{last_restart_slot::LastRestartSlot, SysvarId},
    },
};

// Agave's sysvar cache is difficult to work with, so Mollusk offers a wrapper
// around it for modifying its contents.
/// Mollusk sysvars.
#[derive(Default)]
pub struct MolluskSysvars {
    pub clock: Clock,
    pub epoch_rewards: EpochRewards,
    pub epoch_schedule: EpochSchedule,
    pub last_restart_slot: LastRestartSlot,
    pub rent: Rent,
    pub slot_hashes: SlotHashes,
    pub stake_history: StakeHistory,
}

impl MolluskSysvars {
    /// Warp the test environment to a slot by updating sysvars.
    pub fn warp_to_slot(&mut self, slot: Slot) {
        // First update `Clock`.
        let epoch = self.epoch_schedule.get_epoch(slot);
        let leader_schedule_epoch = self.epoch_schedule.get_leader_schedule_epoch(slot);
        self.clock = Clock {
            slot,
            epoch,
            leader_schedule_epoch,
            ..Default::default()
        };

        // Then update `SlotHashes`.
        let mut i = 0;
        if let Some(most_recent_slot_hash) = self.slot_hashes.first() {
            i = most_recent_slot_hash.0;
        }
        let mut new_slot_hashes = vec![];
        for slot in i..slot + 1 {
            new_slot_hashes.push((slot, Hash::default()));
        }
        self.slot_hashes = SlotHashes::new(&new_slot_hashes);
    }
}

impl From<&MolluskSysvars> for SysvarCache {
    fn from(mollusk_cache: &MolluskSysvars) -> Self {
        let mut sysvar_cache = SysvarCache::default();
        sysvar_cache.fill_missing_entries(|pubkey, set_sysvar| {
            if pubkey.eq(&Clock::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.clock).unwrap());
            }
            if pubkey.eq(&EpochRewards::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.epoch_rewards).unwrap());
            }
            if pubkey.eq(&EpochSchedule::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.epoch_schedule).unwrap());
            }
            if pubkey.eq(&LastRestartSlot::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.last_restart_slot).unwrap());
            }
            if pubkey.eq(&Rent::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.rent).unwrap());
            }
            if pubkey.eq(&SlotHashes::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.slot_hashes).unwrap());
            }
            if pubkey.eq(&StakeHistory::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.stake_history).unwrap());
            }
        });
        sysvar_cache
    }
}

#[cfg(test)]
mod tests {
    use {super::*, solana_sdk::stake_history::StakeHistoryEntry, std::ops::Deref};

    #[test]
    fn test_warp_to_slot() {
        let mut sysvars = MolluskSysvars::default();
        assert_eq!(sysvars.clock.slot, 0);

        sysvars.warp_to_slot(200);
        assert_eq!(sysvars.clock.slot, 200);

        sysvars.warp_to_slot(4_000);
        assert_eq!(sysvars.clock.slot, 4_000);

        sysvars.warp_to_slot(800_000);
        assert_eq!(sysvars.clock.slot, 800_000);
    }

    #[test]
    fn test_to_sysvar_cache() {
        let clock = Clock {
            slot: 1,
            epoch: 2,
            leader_schedule_epoch: 3,
            ..Default::default()
        };
        let epoch_rewards = EpochRewards {
            total_rewards: 4,
            ..Default::default()
        };
        let epoch_schedule = EpochSchedule {
            slots_per_epoch: 5,
            ..Default::default()
        };
        let last_restart_slot = LastRestartSlot {
            last_restart_slot: 6,
        };
        let rent = Rent {
            lamports_per_byte_year: 7,
            ..Default::default()
        };
        let slot_hashes = SlotHashes::new(&[(8, Hash::default())]);
        let stake_history = {
            let mut stake_history = StakeHistory::default();
            stake_history.add(9, StakeHistoryEntry::default());
            stake_history
        };

        let sysvars = MolluskSysvars {
            clock,
            epoch_rewards,
            epoch_schedule,
            last_restart_slot,
            rent,
            slot_hashes,
            stake_history,
        };

        let sysvar_cache: SysvarCache = (&sysvars).into();
        assert_eq!(sysvar_cache.get_clock().unwrap().deref(), &sysvars.clock);
        assert_eq!(
            sysvar_cache.get_epoch_rewards().unwrap().deref(),
            &sysvars.epoch_rewards
        );
        assert_eq!(
            sysvar_cache.get_epoch_schedule().unwrap().deref(),
            &sysvars.epoch_schedule
        );
        assert_eq!(
            sysvar_cache.get_last_restart_slot().unwrap().deref(),
            &sysvars.last_restart_slot
        );
        assert_eq!(sysvar_cache.get_rent().unwrap().deref(), &sysvars.rent);
        assert_eq!(
            sysvar_cache.get_slot_hashes().unwrap().deref(),
            &sysvars.slot_hashes
        );
        assert_eq!(
            sysvar_cache.get_stake_history().unwrap().deref(),
            &sysvars.stake_history
        );
    }
}
