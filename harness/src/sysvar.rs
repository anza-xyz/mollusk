//! Module for working with Solana sysvars.

use {
    solana_account::{Account, ReadableAccount},
    solana_clock::{Clock, Slot},
    solana_epoch_rewards::EpochRewards,
    solana_epoch_schedule::EpochSchedule,
    solana_hash::Hash,
    solana_program_runtime::sysvar_cache::SysvarCache,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    solana_slot_hashes::{SlotHashes, MAX_ENTRIES as SLOT_HASHES_MAX_ENTRIES},
    solana_stake_interface::stake_history::{StakeHistory, StakeHistoryEntry},
    solana_sysvar::{self, last_restart_slot::LastRestartSlot, Sysvar},
    solana_sysvar_id::SysvarId,
};

// Agave's sysvar cache is difficult to work with, so Mollusk offers a wrapper
// around it for modifying its contents.
/// Mollusk sysvars.
pub struct Sysvars {
    pub clock: Clock,
    pub epoch_rewards: EpochRewards,
    pub epoch_schedule: EpochSchedule,
    pub last_restart_slot: LastRestartSlot,
    pub rent: Rent,
    pub slot_hashes: SlotHashes,
    pub stake_history: StakeHistory,
}

impl Default for Sysvars {
    fn default() -> Self {
        let clock = Clock::default();
        let epoch_rewards = EpochRewards::default();
        let epoch_schedule = EpochSchedule::without_warmup();
        let last_restart_slot = LastRestartSlot::default();
        let rent = Rent::default();

        let slot_hashes = {
            let mut default_slot_hashes = vec![(0, Hash::default()); SLOT_HASHES_MAX_ENTRIES];
            default_slot_hashes[0] = (clock.slot, Hash::default());
            SlotHashes::new(&default_slot_hashes)
        };

        let mut stake_history = StakeHistory::default();
        stake_history.add(clock.epoch, StakeHistoryEntry::default());

        Self {
            clock,
            epoch_rewards,
            epoch_schedule,
            last_restart_slot,
            rent,
            slot_hashes,
            stake_history,
        }
    }
}

impl Sysvars {
    fn sysvar_account<T: SysvarId + Sysvar + serde::Serialize>(
        &self,
        sysvar: &T,
    ) -> (Pubkey, Account) {
        let data = bincode::serialize::<T>(sysvar).unwrap();
        let space = data.len();
        let lamports = self.rent.minimum_balance(space);
        let account = Account {
            lamports,
            data,
            owner: solana_sdk_ids::sysvar::id(),
            executable: false,
            ..Default::default()
        };
        (T::id(), account)
    }

    pub(crate) fn maybe_create_sysvar_account(&self, pubkey: &Pubkey) -> Option<Account> {
        if pubkey.eq(&Clock::id()) {
            Some(self.sysvar_account(&self.clock).1)
        } else if pubkey.eq(&EpochRewards::id()) {
            Some(self.sysvar_account(&self.epoch_rewards).1)
        } else if pubkey.eq(&EpochSchedule::id()) {
            Some(self.sysvar_account(&self.epoch_schedule).1)
        } else if pubkey.eq(&LastRestartSlot::id()) {
            Some(self.sysvar_account(&self.last_restart_slot).1)
        } else if pubkey.eq(&Rent::id()) {
            Some(self.sysvar_account(&self.rent).1)
        } else if pubkey.eq(&SlotHashes::id()) {
            Some(self.sysvar_account(&self.slot_hashes).1)
        } else if pubkey.eq(&StakeHistory::id()) {
            Some(self.sysvar_account(&self.stake_history).1)
        } else {
            None
        }
    }

    /// Get the key and account for the clock sysvar.
    pub fn keyed_account_for_clock_sysvar(&self) -> (Pubkey, Account) {
        self.sysvar_account(&self.clock)
    }

    /// Get the key and account for the epoch rewards sysvar.
    pub fn keyed_account_for_epoch_rewards_sysvar(&self) -> (Pubkey, Account) {
        self.sysvar_account(&self.epoch_rewards)
    }

    /// Get the key and account for the epoch schedule sysvar.
    pub fn keyed_account_for_epoch_schedule_sysvar(&self) -> (Pubkey, Account) {
        self.sysvar_account(&self.epoch_schedule)
    }

    /// Get the key and account for the last restart slot sysvar.
    pub fn keyed_account_for_last_restart_slot_sysvar(&self) -> (Pubkey, Account) {
        self.sysvar_account(&self.last_restart_slot)
    }

    /// Get the key and account for the rent sysvar.
    pub fn keyed_account_for_rent_sysvar(&self) -> (Pubkey, Account) {
        self.sysvar_account(&self.rent)
    }

    /// Get the key and account for the slot hashes sysvar.
    pub fn keyed_account_for_slot_hashes_sysvar(&self) -> (Pubkey, Account) {
        self.sysvar_account(&self.slot_hashes)
    }

    /// Get the key and account for the stake history sysvar.
    pub fn keyed_account_for_stake_history_sysvar(&self) -> (Pubkey, Account) {
        self.sysvar_account(&self.stake_history)
    }

    pub(crate) fn get_all_keyed_sysvar_accounts(&self) -> Vec<(Pubkey, Account)> {
        vec![
            self.keyed_account_for_clock_sysvar(),
            self.keyed_account_for_epoch_rewards_sysvar(),
            self.keyed_account_for_epoch_schedule_sysvar(),
            self.keyed_account_for_last_restart_slot_sysvar(),
            self.keyed_account_for_rent_sysvar(),
            self.keyed_account_for_slot_hashes_sysvar(),
            self.keyed_account_for_stake_history_sysvar(),
        ]
    }

    /// Warp the test environment to a slot by updating sysvars.
    pub fn warp_to_slot(&mut self, slot: Slot) {
        let slot_delta = slot.saturating_sub(self.clock.slot);

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
        if slot_delta > SLOT_HASHES_MAX_ENTRIES as u64 {
            let final_hash_slot = slot - SLOT_HASHES_MAX_ENTRIES as u64;

            let slot_hash_entries = (final_hash_slot..slot)
                .rev()
                .map(|slot| (slot, Hash::default()))
                .collect::<Vec<_>>();

            self.slot_hashes = SlotHashes::new(&slot_hash_entries);
        } else {
            let i = if let Some(most_recent_slot_hash) = self.slot_hashes.first() {
                most_recent_slot_hash.0
            } else {
                // By default, this zero is never used, but a user can overwrite
                // `SlotHashes`.
                0
            };
            // Don't include the target slot, since it will become the "current"
            // slot.
            for slot in i..slot {
                self.slot_hashes.add(slot, Hash::default());
            }
        }
    }

    pub(crate) fn setup_sysvar_cache(&self, accounts: &[(Pubkey, Account)]) -> SysvarCache {
        let mut sysvar_cache = SysvarCache::default();

        // First fill any sysvar cache entries from the provided accounts.
        sysvar_cache.fill_missing_entries(|pubkey, set_sysvar| {
            if let Some((_, account)) = accounts.iter().find(|(key, _)| key == pubkey) {
                set_sysvar(account.data())
            }
        });

        // Then fill the rest with the entries from `self`.
        sysvar_cache.fill_missing_entries(|pubkey, set_sysvar| {
            if pubkey.eq(&Clock::id()) {
                set_sysvar(&bincode::serialize(&self.clock).unwrap());
            }
            if pubkey.eq(&EpochRewards::id()) {
                set_sysvar(&bincode::serialize(&self.epoch_rewards).unwrap());
            }
            if pubkey.eq(&EpochSchedule::id()) {
                set_sysvar(&bincode::serialize(&self.epoch_schedule).unwrap());
            }
            if pubkey.eq(&LastRestartSlot::id()) {
                set_sysvar(&bincode::serialize(&self.last_restart_slot).unwrap());
            }
            if pubkey.eq(&Rent::id()) {
                set_sysvar(&bincode::serialize(&self.rent).unwrap());
            }
            if pubkey.eq(&SlotHashes::id()) {
                set_sysvar(&bincode::serialize(&self.slot_hashes).unwrap());
            }
            if pubkey.eq(&StakeHistory::id()) {
                set_sysvar(&bincode::serialize(&self.stake_history).unwrap());
            }
        });

        sysvar_cache
    }
}

impl From<&Sysvars> for SysvarCache {
    fn from(mollusk_cache: &Sysvars) -> Self {
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
    use {super::*, solana_stake_interface::stake_history::StakeHistoryEntry, std::ops::Deref};

    #[test]
    fn test_warp_to_slot() {
        let mut sysvars = Sysvars::default();

        let slot = 0;
        assert_eq!(sysvars.clock.slot, slot);
        assert_eq!(sysvars.clock.epoch, sysvars.epoch_schedule.get_epoch(slot));
        assert_eq!(
            sysvars.slot_hashes.as_slice(),
            &[(slot, Hash::default()); SLOT_HASHES_MAX_ENTRIES]
        );
        assert_eq!(sysvars.slot_hashes.len(), SLOT_HASHES_MAX_ENTRIES);

        let mut warp_and_check = |slot: Slot| {
            sysvars.warp_to_slot(slot);
            assert_eq!(sysvars.clock.slot, slot);
            assert_eq!(sysvars.clock.epoch, sysvars.epoch_schedule.get_epoch(slot));
            assert_eq!(
                sysvars.slot_hashes.first(),
                Some(&(slot - 1, Hash::default())),
            );
            assert_eq!(sysvars.slot_hashes.len(), SLOT_HASHES_MAX_ENTRIES);
        };

        warp_and_check(200);
        warp_and_check(4_000);
        warp_and_check(800_000);
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

        let sysvars = Sysvars {
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
