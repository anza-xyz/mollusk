//! Vote state

#[cfg(feature = "dev-context-only-utils")]
use arbitrary::Arbitrary;
#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};
#[cfg(feature = "frozen-abi")]
use solana_frozen_abi_macro::AbiExample;
use {
    crate::authorized_voters::AuthorizedVoters,
    solana_clock::{Epoch, Slot, UnixTimestamp},
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    std::{collections::VecDeque, fmt::Debug},
};
#[cfg(test)]
use {arbitrary::Unstructured, solana_epoch_schedule::MAX_LEADER_SCHEDULE_EPOCH_OFFSET};

mod vote_state_0_23_5;
pub mod vote_state_1_14_11;
pub use vote_state_1_14_11::*;
pub mod vote_state_versions;
pub use vote_state_versions::*;
pub mod vote_state_v3;
pub use vote_state_v3::VoteStateV3;
pub mod vote_state_v4;
pub use vote_state_v4::VoteStateV4;
mod vote_instruction_data;
pub use vote_instruction_data::*;
#[cfg(any(target_os = "solana", feature = "bincode"))]
pub(crate) mod vote_state_deserialize;

/// Size of a BLS public key in a compressed point representation
pub const BLS_PUBLIC_KEY_COMPRESSED_SIZE: usize = 48;

// Maximum number of votes to keep around, tightly coupled with epoch_schedule::MINIMUM_SLOTS_PER_EPOCH
pub const MAX_LOCKOUT_HISTORY: usize = 31;
pub const INITIAL_LOCKOUT: usize = 2;

// Maximum number of credits history to keep around
pub const MAX_EPOCH_CREDITS_HISTORY: usize = 64;

// Offset of VoteState::prior_voters, for determining initialization status without deserialization
const DEFAULT_PRIOR_VOTERS_OFFSET: usize = 114;

// Number of slots of grace period for which maximum vote credits are awarded - votes landing within this number of slots of the slot that is being voted on are awarded full credits.
pub const VOTE_CREDITS_GRACE_SLOTS: u8 = 2;

// Maximum number of credits to award for a vote; this number of credits is awarded to votes on slots that land within the grace period. After that grace period, vote credits are reduced.
pub const VOTE_CREDITS_MAXIMUM_PER_SLOT: u8 = 16;

#[cfg_attr(feature = "frozen-abi", derive(AbiExample))]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Default, Debug, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(feature = "dev-context-only-utils", derive(Arbitrary))]
pub struct Lockout {
    slot: Slot,
    confirmation_count: u32,
}

impl Lockout {
    pub fn new(slot: Slot) -> Self {
        Self::new_with_confirmation_count(slot, 1)
    }

    pub fn new_with_confirmation_count(slot: Slot, confirmation_count: u32) -> Self {
        Self {
            slot,
            confirmation_count,
        }
    }

    // The number of slots for which this vote is locked
    pub fn lockout(&self) -> u64 {
        (INITIAL_LOCKOUT as u64).wrapping_pow(std::cmp::min(
            self.confirmation_count(),
            MAX_LOCKOUT_HISTORY as u32,
        ))
    }

    // The last slot at which a vote is still locked out. Validators should not
    // vote on a slot in another fork which is less than or equal to this slot
    // to avoid having their stake slashed.
    pub fn last_locked_out_slot(&self) -> Slot {
        self.slot.saturating_add(self.lockout())
    }

    pub fn is_locked_out_at_slot(&self, slot: Slot) -> bool {
        self.last_locked_out_slot() >= slot
    }

    pub fn slot(&self) -> Slot {
        self.slot
    }

    pub fn confirmation_count(&self) -> u32 {
        self.confirmation_count
    }

    pub fn increase_confirmation_count(&mut self, by: u32) {
        self.confirmation_count = self.confirmation_count.saturating_add(by)
    }
}

#[cfg_attr(feature = "frozen-abi", derive(AbiExample))]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Default, Debug, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(feature = "dev-context-only-utils", derive(Arbitrary))]
pub struct LandedVote {
    // Latency is the difference in slot number between the slot that was voted on (lockout.slot) and the slot in
    // which the vote that added this Lockout landed.  For votes which were cast before versions of the validator
    // software which recorded vote latencies, latency is recorded as 0.
    pub latency: u8,
    pub lockout: Lockout,
}

impl LandedVote {
    pub fn slot(&self) -> Slot {
        self.lockout.slot
    }

    pub fn confirmation_count(&self) -> u32 {
        self.lockout.confirmation_count
    }
}

impl From<LandedVote> for Lockout {
    fn from(landed_vote: LandedVote) -> Self {
        landed_vote.lockout
    }
}

impl From<Lockout> for LandedVote {
    fn from(lockout: Lockout) -> Self {
        Self {
            latency: 0,
            lockout,
        }
    }
}

#[cfg_attr(feature = "frozen-abi", derive(AbiExample))]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Debug, Default, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "dev-context-only-utils", derive(Arbitrary))]
pub struct BlockTimestamp {
    pub slot: Slot,
    pub timestamp: UnixTimestamp,
}

// this is how many epochs a voter can be remembered for slashing
const MAX_ITEMS: usize = 32;

#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "frozen-abi", derive(AbiExample))]
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "dev-context-only-utils", derive(Arbitrary))]
pub struct CircBuf<I> {
    buf: [I; MAX_ITEMS],
    /// next pointer
    idx: usize,
    is_empty: bool,
}

impl<I: Default + Copy> Default for CircBuf<I> {
    fn default() -> Self {
        Self {
            buf: [I::default(); MAX_ITEMS],
            idx: MAX_ITEMS
                .checked_sub(1)
                .expect("`MAX_ITEMS` should be positive"),
            is_empty: true,
        }
    }
}

impl<I> CircBuf<I> {
    pub fn append(&mut self, item: I) {
        // remember prior delegate and when we switched, to support later slashing
        self.idx = self
            .idx
            .checked_add(1)
            .and_then(|idx| idx.checked_rem(MAX_ITEMS))
            .expect("`self.idx` should be < `MAX_ITEMS` which should be non-zero");

        self.buf[self.idx] = item;
        self.is_empty = false;
    }

    pub fn buf(&self) -> &[I; MAX_ITEMS] {
        &self.buf
    }

    pub fn last(&self) -> Option<&I> {
        if !self.is_empty {
            self.buf.get(self.idx)
        } else {
            None
        }
    }
}

#[cfg(feature = "serde")]
pub mod serde_compact_vote_state_update {
    use {
        super::*,
        crate::state::Lockout,
        serde::{Deserialize, Deserializer, Serialize, Serializer},
        solana_hash::Hash,
        solana_serde_varint as serde_varint, solana_short_vec as short_vec,
    };

    #[cfg_attr(feature = "frozen-abi", derive(AbiExample))]
    #[derive(serde_derive::Deserialize, serde_derive::Serialize)]
    struct LockoutOffset {
        #[serde(with = "serde_varint")]
        offset: Slot,
        confirmation_count: u8,
    }

    #[derive(serde_derive::Deserialize, serde_derive::Serialize)]
    struct CompactVoteStateUpdate {
        root: Slot,
        #[serde(with = "short_vec")]
        lockout_offsets: Vec<LockoutOffset>,
        hash: Hash,
        timestamp: Option<UnixTimestamp>,
    }

    pub fn serialize<S>(
        vote_state_update: &VoteStateUpdate,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let lockout_offsets = vote_state_update.lockouts.iter().scan(
            vote_state_update.root.unwrap_or_default(),
            |slot, lockout| {
                let Some(offset) = lockout.slot().checked_sub(*slot) else {
                    return Some(Err(serde::ser::Error::custom("Invalid vote lockout")));
                };
                let Ok(confirmation_count) = u8::try_from(lockout.confirmation_count()) else {
                    return Some(Err(serde::ser::Error::custom("Invalid confirmation count")));
                };
                let lockout_offset = LockoutOffset {
                    offset,
                    confirmation_count,
                };
                *slot = lockout.slot();
                Some(Ok(lockout_offset))
            },
        );
        let compact_vote_state_update = CompactVoteStateUpdate {
            root: vote_state_update.root.unwrap_or(Slot::MAX),
            lockout_offsets: lockout_offsets.collect::<Result<_, _>>()?,
            hash: Hash::new_from_array(vote_state_update.hash.to_bytes()),
            timestamp: vote_state_update.timestamp,
        };
        compact_vote_state_update.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<VoteStateUpdate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let CompactVoteStateUpdate {
            root,
            lockout_offsets,
            hash,
            timestamp,
        } = CompactVoteStateUpdate::deserialize(deserializer)?;
        let root = (root != Slot::MAX).then_some(root);
        let lockouts =
            lockout_offsets
                .iter()
                .scan(root.unwrap_or_default(), |slot, lockout_offset| {
                    *slot = match slot.checked_add(lockout_offset.offset) {
                        None => {
                            return Some(Err(serde::de::Error::custom("Invalid lockout offset")))
                        }
                        Some(slot) => slot,
                    };
                    let lockout = Lockout::new_with_confirmation_count(
                        *slot,
                        u32::from(lockout_offset.confirmation_count),
                    );
                    Some(Ok(lockout))
                });
        Ok(VoteStateUpdate {
            root,
            lockouts: lockouts.collect::<Result<_, _>>()?,
            hash,
            timestamp,
        })
    }
}

#[cfg(feature = "serde")]
pub mod serde_tower_sync {
    use {
        super::*,
        crate::state::Lockout,
        serde::{Deserialize, Deserializer, Serialize, Serializer},
        solana_hash::Hash,
        solana_serde_varint as serde_varint, solana_short_vec as short_vec,
    };

    #[cfg_attr(feature = "frozen-abi", derive(AbiExample))]
    #[derive(serde_derive::Deserialize, serde_derive::Serialize)]
    struct LockoutOffset {
        #[serde(with = "serde_varint")]
        offset: Slot,
        confirmation_count: u8,
    }

    #[derive(serde_derive::Deserialize, serde_derive::Serialize)]
    struct CompactTowerSync {
        root: Slot,
        #[serde(with = "short_vec")]
        lockout_offsets: Vec<LockoutOffset>,
        hash: Hash,
        timestamp: Option<UnixTimestamp>,
        block_id: Hash,
    }

    pub fn serialize<S>(tower_sync: &TowerSync, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let lockout_offsets = tower_sync.lockouts.iter().scan(
            tower_sync.root.unwrap_or_default(),
            |slot, lockout| {
                let Some(offset) = lockout.slot().checked_sub(*slot) else {
                    return Some(Err(serde::ser::Error::custom("Invalid vote lockout")));
                };
                let Ok(confirmation_count) = u8::try_from(lockout.confirmation_count()) else {
                    return Some(Err(serde::ser::Error::custom("Invalid confirmation count")));
                };
                let lockout_offset = LockoutOffset {
                    offset,
                    confirmation_count,
                };
                *slot = lockout.slot();
                Some(Ok(lockout_offset))
            },
        );
        let compact_tower_sync = CompactTowerSync {
            root: tower_sync.root.unwrap_or(Slot::MAX),
            lockout_offsets: lockout_offsets.collect::<Result<_, _>>()?,
            hash: Hash::new_from_array(tower_sync.hash.to_bytes()),
            timestamp: tower_sync.timestamp,
            block_id: Hash::new_from_array(tower_sync.block_id.to_bytes()),
        };
        compact_tower_sync.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<TowerSync, D::Error>
    where
        D: Deserializer<'de>,
    {
        let CompactTowerSync {
            root,
            lockout_offsets,
            hash,
            timestamp,
            block_id,
        } = CompactTowerSync::deserialize(deserializer)?;
        let root = (root != Slot::MAX).then_some(root);
        let lockouts =
            lockout_offsets
                .iter()
                .scan(root.unwrap_or_default(), |slot, lockout_offset| {
                    *slot = match slot.checked_add(lockout_offset.offset) {
                        None => {
                            return Some(Err(serde::de::Error::custom("Invalid lockout offset")))
                        }
                        Some(slot) => slot,
                    };
                    let lockout = Lockout::new_with_confirmation_count(
                        *slot,
                        u32::from(lockout_offset.confirmation_count),
                    );
                    Some(Ok(lockout))
                });
        Ok(TowerSync {
            root,
            lockouts: lockouts.collect::<Result<_, _>>()?,
            hash,
            timestamp,
            block_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{error::VoteError, state::vote_state_0_23_5::VoteState0_23_5},
        bincode::serialized_size,
        core::mem::MaybeUninit,
        itertools::Itertools,
        rand::Rng,
        solana_clock::Clock,
        solana_hash::Hash,
        solana_instruction::error::InstructionError,
    };

    // Test helper to create a VoteStateV4 with random data for testing
    fn create_test_vote_state_v4(node_pubkey: Pubkey, root_slot: Slot) -> VoteStateV4 {
        let votes = (1..32)
            .map(|x| LandedVote {
                latency: 0,
                lockout: Lockout::new_with_confirmation_count(
                    u64::from(x).saturating_add(root_slot),
                    32_u32.saturating_sub(x),
                ),
            })
            .collect();
        VoteStateV4 {
            node_pubkey,
            root_slot: Some(root_slot),
            votes,
            ..VoteStateV4::default()
        }
    }

    #[test]
    fn test_vote_serialize_v3() {
        let mut buffer: Vec<u8> = vec![0; VoteStateV3::size_of()];
        let mut vote_state = VoteStateV3::default();
        vote_state
            .votes
            .resize(MAX_LOCKOUT_HISTORY, LandedVote::default());
        vote_state.root_slot = Some(1);
        let versioned = VoteStateVersions::new_v3(vote_state);
        assert!(VoteStateV3::serialize(&versioned, &mut buffer[0..4]).is_err());
        VoteStateV3::serialize(&versioned, &mut buffer).unwrap();
        assert_eq!(
            VoteStateV3::deserialize(&buffer).unwrap(),
            versioned.try_convert_to_v3().unwrap()
        );
    }

    #[test]
    fn test_vote_serialize_v4() {
        // Use two different pubkeys to demonstrate that v4 ignores the
        // `vote_pubkey` parameter.
        let vote_pubkey_for_deserialize = Pubkey::new_unique();
        let vote_pubkey_for_convert = Pubkey::new_unique();

        let mut buffer: Vec<u8> = vec![0; VoteStateV4::size_of()];
        let mut vote_state = VoteStateV4::default();
        vote_state
            .votes
            .resize(MAX_LOCKOUT_HISTORY, LandedVote::default());
        vote_state.root_slot = Some(1);
        let versioned = VoteStateVersions::new_v4(vote_state);
        assert!(VoteStateV4::serialize(&versioned, &mut buffer[0..4]).is_err());
        VoteStateV4::serialize(&versioned, &mut buffer).unwrap();
        assert_eq!(
            VoteStateV4::deserialize(&buffer, &vote_pubkey_for_deserialize).unwrap(),
            versioned
                .try_convert_to_v4(&vote_pubkey_for_convert)
                .unwrap()
        );
    }

    #[test]
    fn test_vote_deserialize_into_v3() {
        // base case
        let target_vote_state = VoteStateV3::default();
        let vote_state_buf =
            bincode::serialize(&VoteStateVersions::new_v3(target_vote_state.clone())).unwrap();

        let mut test_vote_state = VoteStateV3::default();
        VoteStateV3::deserialize_into(&vote_state_buf, &mut test_vote_state).unwrap();

        assert_eq!(target_vote_state, test_vote_state);

        // variant
        // provide 4x the minimum struct size in bytes to ensure we typically touch every field
        let struct_bytes_x4 = std::mem::size_of::<VoteStateV3>() * 4;
        for _ in 0..1000 {
            let raw_data: Vec<u8> = (0..struct_bytes_x4).map(|_| rand::random::<u8>()).collect();
            let mut unstructured = Unstructured::new(&raw_data);

            let target_vote_state_versions =
                VoteStateVersions::arbitrary(&mut unstructured).unwrap();
            let vote_state_buf = bincode::serialize(&target_vote_state_versions).unwrap();

            // Skip any v4 since they can't convert to v3.
            if let Ok(target_vote_state) = target_vote_state_versions.try_convert_to_v3() {
                let mut test_vote_state = VoteStateV3::default();
                VoteStateV3::deserialize_into(&vote_state_buf, &mut test_vote_state).unwrap();

                assert_eq!(target_vote_state, test_vote_state);
            }
        }
    }

    #[test]
    fn test_vote_deserialize_into_v4() {
        let vote_pubkey = Pubkey::new_unique();

        // base case
        let target_vote_state = VoteStateV4::default();
        let vote_state_buf =
            bincode::serialize(&VoteStateVersions::new_v4(target_vote_state.clone())).unwrap();

        let mut test_vote_state = VoteStateV4::default();
        VoteStateV4::deserialize_into(&vote_state_buf, &mut test_vote_state, &vote_pubkey).unwrap();

        assert_eq!(target_vote_state, test_vote_state);

        // variant
        // provide 4x the minimum struct size in bytes to ensure we typically touch every field
        let struct_bytes_x4 = std::mem::size_of::<VoteStateV4>() * 4;
        for _ in 0..1000 {
            let raw_data: Vec<u8> = (0..struct_bytes_x4).map(|_| rand::random::<u8>()).collect();
            let mut unstructured = Unstructured::new(&raw_data);

            let target_vote_state_versions =
                VoteStateVersions::arbitrary(&mut unstructured).unwrap();
            let vote_state_buf = bincode::serialize(&target_vote_state_versions).unwrap();
            let target_vote_state = target_vote_state_versions
                .try_convert_to_v4(&vote_pubkey)
                .unwrap();

            let mut test_vote_state = VoteStateV4::default();
            VoteStateV4::deserialize_into(&vote_state_buf, &mut test_vote_state, &vote_pubkey)
                .unwrap();

            assert_eq!(target_vote_state, test_vote_state);
        }
    }

    #[test]
    fn test_vote_deserialize_into_error_v3() {
        let target_vote_state = VoteStateV3::new_rand_for_tests(Pubkey::new_unique(), 42);
        let mut vote_state_buf =
            bincode::serialize(&VoteStateVersions::new_v3(target_vote_state.clone())).unwrap();
        let len = vote_state_buf.len();
        vote_state_buf.truncate(len - 1);

        let mut test_vote_state = VoteStateV3::default();
        VoteStateV3::deserialize_into(&vote_state_buf, &mut test_vote_state).unwrap_err();
        assert_eq!(test_vote_state, VoteStateV3::default());
    }

    #[test]
    fn test_vote_deserialize_into_error_v4() {
        let vote_pubkey = Pubkey::new_unique();

        let target_vote_state = create_test_vote_state_v4(Pubkey::new_unique(), 42);
        let mut vote_state_buf =
            bincode::serialize(&VoteStateVersions::new_v4(target_vote_state.clone())).unwrap();
        let len = vote_state_buf.len();
        vote_state_buf.truncate(len - 1);

        let mut test_vote_state = VoteStateV4::default();
        VoteStateV4::deserialize_into(&vote_state_buf, &mut test_vote_state, &vote_pubkey)
            .unwrap_err();
        assert_eq!(test_vote_state, VoteStateV4::default());
    }

    #[test]
    fn test_vote_deserialize_into_uninit_v3() {
        // base case
        let target_vote_state = VoteStateV3::default();
        let vote_state_buf =
            bincode::serialize(&VoteStateVersions::new_v3(target_vote_state.clone())).unwrap();

        let mut test_vote_state = MaybeUninit::uninit();
        VoteStateV3::deserialize_into_uninit(&vote_state_buf, &mut test_vote_state).unwrap();
        let test_vote_state = unsafe { test_vote_state.assume_init() };

        assert_eq!(target_vote_state, test_vote_state);

        // variant
        // provide 4x the minimum struct size in bytes to ensure we typically touch every field
        let struct_bytes_x4 = std::mem::size_of::<VoteStateV3>() * 4;
        for _ in 0..1000 {
            let raw_data: Vec<u8> = (0..struct_bytes_x4).map(|_| rand::random::<u8>()).collect();
            let mut unstructured = Unstructured::new(&raw_data);

            let target_vote_state_versions =
                VoteStateVersions::arbitrary(&mut unstructured).unwrap();
            let vote_state_buf = bincode::serialize(&target_vote_state_versions).unwrap();

            // Skip any v4 since they can't convert to v3.
            if let Ok(target_vote_state) = target_vote_state_versions.try_convert_to_v3() {
                let mut test_vote_state = MaybeUninit::uninit();
                VoteStateV3::deserialize_into_uninit(&vote_state_buf, &mut test_vote_state)
                    .unwrap();
                let test_vote_state = unsafe { test_vote_state.assume_init() };

                assert_eq!(target_vote_state, test_vote_state);
            }
        }
    }

    #[test]
    fn test_vote_deserialize_into_uninit_v4() {
        let vote_pubkey = Pubkey::new_unique();

        // base case
        let target_vote_state = VoteStateV4::default();
        let vote_state_buf =
            bincode::serialize(&VoteStateVersions::new_v4(target_vote_state.clone())).unwrap();

        let mut test_vote_state = MaybeUninit::uninit();
        VoteStateV4::deserialize_into_uninit(&vote_state_buf, &mut test_vote_state, &vote_pubkey)
            .unwrap();
        let test_vote_state = unsafe { test_vote_state.assume_init() };

        assert_eq!(target_vote_state, test_vote_state);

        // variant
        // provide 4x the minimum struct size in bytes to ensure we typically touch every field
        let struct_bytes_x4 = std::mem::size_of::<VoteStateV4>() * 4;
        for _ in 0..1000 {
            let raw_data: Vec<u8> = (0..struct_bytes_x4).map(|_| rand::random::<u8>()).collect();
            let mut unstructured = Unstructured::new(&raw_data);

            let target_vote_state_versions =
                VoteStateVersions::arbitrary(&mut unstructured).unwrap();
            let vote_state_buf = bincode::serialize(&target_vote_state_versions).unwrap();
            let target_vote_state = target_vote_state_versions
                .try_convert_to_v4(&Pubkey::default())
                .unwrap();

            let mut test_vote_state = MaybeUninit::uninit();
            VoteStateV4::deserialize_into_uninit(
                &vote_state_buf,
                &mut test_vote_state,
                &Pubkey::default(),
            )
            .unwrap();
            let test_vote_state = unsafe { test_vote_state.assume_init() };

            assert_eq!(target_vote_state, test_vote_state);
        }
    }

    #[test]
    fn test_vote_deserialize_into_uninit_nopanic_v3() {
        // base case
        let mut test_vote_state = MaybeUninit::uninit();
        let e = VoteStateV3::deserialize_into_uninit(&[], &mut test_vote_state).unwrap_err();
        assert_eq!(e, InstructionError::InvalidAccountData);

        // variant
        let serialized_len_x4 = serialized_size(&VoteStateV3::default()).unwrap() * 4;
        let mut rng = rand::thread_rng();
        for _ in 0..1000 {
            let raw_data_length = rng.gen_range(1..serialized_len_x4);
            let mut raw_data: Vec<u8> = (0..raw_data_length).map(|_| rng.gen::<u8>()).collect();

            // pure random data will ~never have a valid enum tag, so lets help it out
            if raw_data_length >= 4 && rng.gen::<bool>() {
                let tag = rng.gen::<u8>() % 4;
                raw_data[0] = tag;
                raw_data[1] = 0;
                raw_data[2] = 0;
                raw_data[3] = 0;
            }

            // it is extremely improbable, though theoretically possible, for random bytes to be syntactically valid
            // so we only check that the parser does not panic and that it succeeds or fails exactly in line with bincode
            let mut test_vote_state = MaybeUninit::uninit();
            let test_res = VoteStateV3::deserialize_into_uninit(&raw_data, &mut test_vote_state);

            // Test with bincode for consistency.
            let bincode_res = bincode::deserialize::<VoteStateVersions>(&raw_data)
                .map_err(|_| InstructionError::InvalidAccountData)
                .and_then(|versioned| versioned.try_convert_to_v3());

            if test_res.is_err() {
                assert!(bincode_res.is_err());
            } else {
                let test_vote_state = unsafe { test_vote_state.assume_init() };
                assert_eq!(test_vote_state, bincode_res.unwrap());
            }
        }
    }

    #[test]
    fn test_vote_deserialize_into_uninit_nopanic_v4() {
        let vote_pubkey = Pubkey::new_unique();

        // base case
        let mut test_vote_state = MaybeUninit::uninit();
        let e = VoteStateV4::deserialize_into_uninit(&[], &mut test_vote_state, &vote_pubkey)
            .unwrap_err();
        assert_eq!(e, InstructionError::InvalidAccountData);

        // variant
        let serialized_len_x4 = serialized_size(&VoteStateV4::default()).unwrap() * 4;
        let mut rng = rand::thread_rng();
        for _ in 0..1000 {
            let raw_data_length = rng.gen_range(1..serialized_len_x4);
            let mut raw_data: Vec<u8> = (0..raw_data_length).map(|_| rng.gen::<u8>()).collect();

            // pure random data will ~never have a valid enum tag, so lets help it out
            if raw_data_length >= 4 && rng.gen::<bool>() {
                let tag = rng.gen::<u8>() % 4;
                raw_data[0] = tag;
                raw_data[1] = 0;
                raw_data[2] = 0;
                raw_data[3] = 0;
            }

            // it is extremely improbable, though theoretically possible, for random bytes to be syntactically valid
            // so we only check that the parser does not panic and that it succeeds or fails exactly in line with bincode
            let mut test_vote_state = MaybeUninit::uninit();
            let test_res =
                VoteStateV4::deserialize_into_uninit(&raw_data, &mut test_vote_state, &vote_pubkey);
            let bincode_res = bincode::deserialize::<VoteStateVersions>(&raw_data)
                .map(|versioned| versioned.try_convert_to_v4(&vote_pubkey).unwrap());

            if test_res.is_err() {
                assert!(bincode_res.is_err());
            } else {
                let test_vote_state = unsafe { test_vote_state.assume_init() };
                assert_eq!(test_vote_state, bincode_res.unwrap());
            }
        }
    }

    #[test]
    fn test_vote_deserialize_into_uninit_ill_sized_v3() {
        // provide 4x the minimum struct size in bytes to ensure we typically touch every field
        let struct_bytes_x4 = std::mem::size_of::<VoteStateV3>() * 4;
        for _ in 0..1000 {
            let raw_data: Vec<u8> = (0..struct_bytes_x4).map(|_| rand::random::<u8>()).collect();
            let mut unstructured = Unstructured::new(&raw_data);

            let original_vote_state_versions =
                VoteStateVersions::arbitrary(&mut unstructured).unwrap();
            let original_buf = bincode::serialize(&original_vote_state_versions).unwrap();

            // Skip any v4 since they can't convert to v3.
            if !matches!(original_vote_state_versions, VoteStateVersions::V4(_)) {
                let mut truncated_buf = original_buf.clone();
                let mut expanded_buf = original_buf.clone();

                truncated_buf.resize(original_buf.len() - 8, 0);
                expanded_buf.resize(original_buf.len() + 8, 0);

                // truncated fails
                let mut test_vote_state = MaybeUninit::uninit();
                let test_res =
                    VoteStateV3::deserialize_into_uninit(&truncated_buf, &mut test_vote_state);
                // `deserialize_into_uninit` will eventually call into
                // `try_convert_to_v3`, so we have alignment in the following map.
                let bincode_res = bincode::deserialize::<VoteStateVersions>(&truncated_buf)
                    .map_err(|_| InstructionError::InvalidAccountData)
                    .and_then(|versioned| versioned.try_convert_to_v3());

                assert!(test_res.is_err());
                assert!(bincode_res.is_err());

                // expanded succeeds
                let mut test_vote_state = MaybeUninit::uninit();
                VoteStateV3::deserialize_into_uninit(&expanded_buf, &mut test_vote_state).unwrap();
                // `deserialize_into_uninit` will eventually call into
                // `try_convert_to_v3`, so we have alignment in the following map.
                let bincode_res = bincode::deserialize::<VoteStateVersions>(&expanded_buf)
                    .map_err(|_| InstructionError::InvalidAccountData)
                    .and_then(|versioned| versioned.try_convert_to_v3());

                let test_vote_state = unsafe { test_vote_state.assume_init() };
                assert_eq!(test_vote_state, bincode_res.unwrap());
            }
        }
    }

    #[test]
    fn test_vote_deserialize_into_uninit_ill_sized_v4() {
        let vote_pubkey = Pubkey::new_unique();

        // provide 4x the minimum struct size in bytes to ensure we typically touch every field
        let struct_bytes_x4 = std::mem::size_of::<VoteStateV4>() * 4;
        for _ in 0..1000 {
            let raw_data: Vec<u8> = (0..struct_bytes_x4).map(|_| rand::random::<u8>()).collect();
            let mut unstructured = Unstructured::new(&raw_data);

            let original_vote_state_versions =
                VoteStateVersions::arbitrary(&mut unstructured).unwrap();
            let original_buf = bincode::serialize(&original_vote_state_versions).unwrap();

            let mut truncated_buf = original_buf.clone();
            let mut expanded_buf = original_buf.clone();

            truncated_buf.resize(original_buf.len() - 8, 0);
            expanded_buf.resize(original_buf.len() + 8, 0);

            // truncated fails
            let mut test_vote_state = MaybeUninit::uninit();
            let test_res = VoteStateV4::deserialize_into_uninit(
                &truncated_buf,
                &mut test_vote_state,
                &vote_pubkey,
            );
            let bincode_res = bincode::deserialize::<VoteStateVersions>(&truncated_buf)
                .map(|versioned| versioned.try_convert_to_v4(&vote_pubkey).unwrap());

            assert!(test_res.is_err());
            assert!(bincode_res.is_err());

            // expanded succeeds
            let mut test_vote_state = MaybeUninit::uninit();
            VoteStateV4::deserialize_into_uninit(&expanded_buf, &mut test_vote_state, &vote_pubkey)
                .unwrap();
            let bincode_res = bincode::deserialize::<VoteStateVersions>(&expanded_buf)
                .map(|versioned| versioned.try_convert_to_v4(&vote_pubkey).unwrap());

            let test_vote_state = unsafe { test_vote_state.assume_init() };
            assert_eq!(test_vote_state, bincode_res.unwrap());
        }
    }

    #[test]
    fn test_vote_state_epoch_credits() {
        let mut vote_state = VoteStateV3::default();

        assert_eq!(vote_state.credits(), 0);
        assert_eq!(vote_state.epoch_credits().clone(), vec![]);

        let mut expected = vec![];
        let mut credits = 0;
        let epochs = (MAX_EPOCH_CREDITS_HISTORY + 2) as u64;
        for epoch in 0..epochs {
            for _j in 0..epoch {
                vote_state.increment_credits(epoch, 1);
                credits += 1;
            }
            expected.push((epoch, credits, credits - epoch));
        }

        while expected.len() > MAX_EPOCH_CREDITS_HISTORY {
            expected.remove(0);
        }

        assert_eq!(vote_state.credits(), credits);
        assert_eq!(vote_state.epoch_credits().clone(), expected);
    }

    #[test]
    fn test_vote_state_epoch0_no_credits() {
        let mut vote_state = VoteStateV3::default();

        assert_eq!(vote_state.epoch_credits().len(), 0);
        vote_state.increment_credits(1, 1);
        assert_eq!(vote_state.epoch_credits().len(), 1);

        vote_state.increment_credits(2, 1);
        assert_eq!(vote_state.epoch_credits().len(), 2);
    }

    #[test]
    fn test_vote_state_increment_credits() {
        let mut vote_state = VoteStateV3::default();

        let credits = (MAX_EPOCH_CREDITS_HISTORY + 2) as u64;
        for i in 0..credits {
            vote_state.increment_credits(i, 1);
        }
        assert_eq!(vote_state.credits(), credits);
        assert!(vote_state.epoch_credits().len() <= MAX_EPOCH_CREDITS_HISTORY);
    }

    #[test]
    fn test_vote_process_timestamp() {
        let (slot, timestamp) = (15, 1_575_412_285);
        let mut vote_state = VoteStateV3 {
            last_timestamp: BlockTimestamp { slot, timestamp },
            ..VoteStateV3::default()
        };

        assert_eq!(
            vote_state.process_timestamp(slot - 1, timestamp + 1),
            Err(VoteError::TimestampTooOld)
        );
        assert_eq!(
            vote_state.last_timestamp,
            BlockTimestamp { slot, timestamp }
        );
        assert_eq!(
            vote_state.process_timestamp(slot + 1, timestamp - 1),
            Err(VoteError::TimestampTooOld)
        );
        assert_eq!(
            vote_state.process_timestamp(slot, timestamp + 1),
            Err(VoteError::TimestampTooOld)
        );
        assert_eq!(vote_state.process_timestamp(slot, timestamp), Ok(()));
        assert_eq!(
            vote_state.last_timestamp,
            BlockTimestamp { slot, timestamp }
        );
        assert_eq!(vote_state.process_timestamp(slot + 1, timestamp), Ok(()));
        assert_eq!(
            vote_state.last_timestamp,
            BlockTimestamp {
                slot: slot + 1,
                timestamp
            }
        );
        assert_eq!(
            vote_state.process_timestamp(slot + 2, timestamp + 1),
            Ok(())
        );
        assert_eq!(
            vote_state.last_timestamp,
            BlockTimestamp {
                slot: slot + 2,
                timestamp: timestamp + 1
            }
        );

        // Test initial vote
        vote_state.last_timestamp = BlockTimestamp::default();
        assert_eq!(vote_state.process_timestamp(0, timestamp), Ok(()));
    }

    #[test]
    fn test_get_and_update_authorized_voter() {
        let original_voter = Pubkey::new_unique();
        let mut vote_state = VoteStateV3::new(
            &VoteInit {
                node_pubkey: original_voter,
                authorized_voter: original_voter,
                authorized_withdrawer: original_voter,
                commission: 0,
            },
            &Clock::default(),
        );

        assert_eq!(vote_state.authorized_voters.len(), 1);
        assert_eq!(
            *vote_state.authorized_voters.first().unwrap().1,
            original_voter
        );

        // If no new authorized voter was set, the same authorized voter
        // is locked into the next epoch
        assert_eq!(
            vote_state.get_and_update_authorized_voter(1).unwrap(),
            original_voter
        );

        // Try to get the authorized voter for epoch 5, implies
        // the authorized voter for epochs 1-4 were unchanged
        assert_eq!(
            vote_state.get_and_update_authorized_voter(5).unwrap(),
            original_voter
        );

        // Authorized voter for expired epoch 0..5 should have been
        // purged and no longer queryable
        assert_eq!(vote_state.authorized_voters.len(), 1);
        for i in 0..5 {
            assert!(vote_state
                .authorized_voters
                .get_authorized_voter(i)
                .is_none());
        }

        // Set an authorized voter change at slot 7
        let new_authorized_voter = Pubkey::new_unique();
        vote_state
            .set_new_authorized_voter(&new_authorized_voter, 5, 7, |_| Ok(()))
            .unwrap();

        // Try to get the authorized voter for epoch 6, unchanged
        assert_eq!(
            vote_state.get_and_update_authorized_voter(6).unwrap(),
            original_voter
        );

        // Try to get the authorized voter for epoch 7 and onwards, should
        // be the new authorized voter
        for i in 7..10 {
            assert_eq!(
                vote_state.get_and_update_authorized_voter(i).unwrap(),
                new_authorized_voter
            );
        }
        assert_eq!(vote_state.authorized_voters.len(), 1);
    }

    #[test]
    fn test_set_new_authorized_voter() {
        let original_voter = Pubkey::new_unique();
        let epoch_offset = 15;
        let mut vote_state = VoteStateV3::new(
            &VoteInit {
                node_pubkey: original_voter,
                authorized_voter: original_voter,
                authorized_withdrawer: original_voter,
                commission: 0,
            },
            &Clock::default(),
        );

        assert!(vote_state.prior_voters.last().is_none());

        let new_voter = Pubkey::new_unique();
        // Set a new authorized voter
        vote_state
            .set_new_authorized_voter(&new_voter, 0, epoch_offset, |_| Ok(()))
            .unwrap();

        assert_eq!(vote_state.prior_voters.idx, 0);
        assert_eq!(
            vote_state.prior_voters.last(),
            Some(&(original_voter, 0, epoch_offset))
        );

        // Trying to set authorized voter for same epoch again should fail
        assert_eq!(
            vote_state.set_new_authorized_voter(&new_voter, 0, epoch_offset, |_| Ok(())),
            Err(VoteError::TooSoonToReauthorize.into())
        );

        // Setting the same authorized voter again should succeed
        vote_state
            .set_new_authorized_voter(&new_voter, 2, 2 + epoch_offset, |_| Ok(()))
            .unwrap();

        // Set a third and fourth authorized voter
        let new_voter2 = Pubkey::new_unique();
        vote_state
            .set_new_authorized_voter(&new_voter2, 3, 3 + epoch_offset, |_| Ok(()))
            .unwrap();
        assert_eq!(vote_state.prior_voters.idx, 1);
        assert_eq!(
            vote_state.prior_voters.last(),
            Some(&(new_voter, epoch_offset, 3 + epoch_offset))
        );

        let new_voter3 = Pubkey::new_unique();
        vote_state
            .set_new_authorized_voter(&new_voter3, 6, 6 + epoch_offset, |_| Ok(()))
            .unwrap();
        assert_eq!(vote_state.prior_voters.idx, 2);
        assert_eq!(
            vote_state.prior_voters.last(),
            Some(&(new_voter2, 3 + epoch_offset, 6 + epoch_offset))
        );

        // Check can set back to original voter
        vote_state
            .set_new_authorized_voter(&original_voter, 9, 9 + epoch_offset, |_| Ok(()))
            .unwrap();

        // Run with these voters for a while, check the ranges of authorized
        // voters is correct
        for i in 9..epoch_offset {
            assert_eq!(
                vote_state.get_and_update_authorized_voter(i).unwrap(),
                original_voter
            );
        }
        for i in epoch_offset..3 + epoch_offset {
            assert_eq!(
                vote_state.get_and_update_authorized_voter(i).unwrap(),
                new_voter
            );
        }
        for i in 3 + epoch_offset..6 + epoch_offset {
            assert_eq!(
                vote_state.get_and_update_authorized_voter(i).unwrap(),
                new_voter2
            );
        }
        for i in 6 + epoch_offset..9 + epoch_offset {
            assert_eq!(
                vote_state.get_and_update_authorized_voter(i).unwrap(),
                new_voter3
            );
        }
        for i in 9 + epoch_offset..=10 + epoch_offset {
            assert_eq!(
                vote_state.get_and_update_authorized_voter(i).unwrap(),
                original_voter
            );
        }
    }

    #[test]
    fn test_authorized_voter_is_locked_within_epoch() {
        let original_voter = Pubkey::new_unique();
        let mut vote_state = VoteStateV3::new(
            &VoteInit {
                node_pubkey: original_voter,
                authorized_voter: original_voter,
                authorized_withdrawer: original_voter,
                commission: 0,
            },
            &Clock::default(),
        );

        // Test that it's not possible to set a new authorized
        // voter within the same epoch, even if none has been
        // explicitly set before
        let new_voter = Pubkey::new_unique();
        assert_eq!(
            vote_state.set_new_authorized_voter(&new_voter, 1, 1, |_| Ok(())),
            Err(VoteError::TooSoonToReauthorize.into())
        );

        assert_eq!(vote_state.get_authorized_voter(1), Some(original_voter));

        // Set a new authorized voter for a future epoch
        assert_eq!(
            vote_state.set_new_authorized_voter(&new_voter, 1, 2, |_| Ok(())),
            Ok(())
        );

        // Test that it's not possible to set a new authorized
        // voter within the same epoch, even if none has been
        // explicitly set before
        assert_eq!(
            vote_state.set_new_authorized_voter(&original_voter, 3, 3, |_| Ok(())),
            Err(VoteError::TooSoonToReauthorize.into())
        );

        assert_eq!(vote_state.get_authorized_voter(3), Some(new_voter));
    }

    #[test]
    fn test_vote_state_v3_size_of() {
        let vote_state = VoteStateV3::get_max_sized_vote_state();
        let vote_state = VoteStateVersions::new_v3(vote_state);
        let size = serialized_size(&vote_state).unwrap();
        assert_eq!(VoteStateV3::size_of() as u64, size);
    }

    #[test]
    fn test_vote_state_v4_size_of() {
        let vote_state = VoteStateV4::get_max_sized_vote_state();
        let vote_state = VoteStateVersions::new_v4(vote_state);
        let size = serialized_size(&vote_state).unwrap();
        assert!(size < VoteStateV4::size_of() as u64); // v4 is smaller than the max size
    }

    #[test]
    fn test_vote_state_max_size() {
        let mut max_sized_data = vec![0; VoteStateV3::size_of()];
        let vote_state = VoteStateV3::get_max_sized_vote_state();
        let (start_leader_schedule_epoch, _) = vote_state.authorized_voters.last().unwrap();
        let start_current_epoch =
            start_leader_schedule_epoch - MAX_LEADER_SCHEDULE_EPOCH_OFFSET + 1;

        let mut vote_state = Some(vote_state);
        for i in start_current_epoch..start_current_epoch + 2 * MAX_LEADER_SCHEDULE_EPOCH_OFFSET {
            vote_state.as_mut().map(|vote_state| {
                vote_state.set_new_authorized_voter(
                    &Pubkey::new_unique(),
                    i,
                    i + MAX_LEADER_SCHEDULE_EPOCH_OFFSET,
                    |_| Ok(()),
                )
            });

            let versioned = VoteStateVersions::new_v3(vote_state.take().unwrap());
            VoteStateV3::serialize(&versioned, &mut max_sized_data).unwrap();
            vote_state = Some(versioned.try_convert_to_v3().unwrap());
        }
    }

    #[test]
    fn test_default_vote_state_is_uninitialized() {
        // The default `VoteStateV3` is stored to de-initialize a zero-balance vote account,
        // so must remain such that `VoteStateVersions::is_uninitialized()` returns true
        // when called on a `VoteStateVersions` that stores it
        assert!(VoteStateVersions::new_v3(VoteStateV3::default()).is_uninitialized());
    }

    #[test]
    fn test_is_correct_size_and_initialized() {
        // Check all zeroes
        let mut vote_account_data = vec![0; VoteStateV3::size_of()];
        assert!(!VoteStateVersions::is_correct_size_and_initialized(
            &vote_account_data
        ));

        // Check default VoteStateV3
        let default_account_state = VoteStateVersions::new_v3(VoteStateV3::default());
        VoteStateV3::serialize(&default_account_state, &mut vote_account_data).unwrap();
        assert!(!VoteStateVersions::is_correct_size_and_initialized(
            &vote_account_data
        ));

        // Check non-zero data shorter than offset index used
        let short_data = vec![1; DEFAULT_PRIOR_VOTERS_OFFSET];
        assert!(!VoteStateVersions::is_correct_size_and_initialized(
            &short_data
        ));

        // Check non-zero large account
        let mut large_vote_data = vec![1; 2 * VoteStateV3::size_of()];
        let default_account_state = VoteStateVersions::new_v3(VoteStateV3::default());
        VoteStateV3::serialize(&default_account_state, &mut large_vote_data).unwrap();
        assert!(!VoteStateVersions::is_correct_size_and_initialized(
            &vote_account_data
        ));

        // Check populated VoteStateV3
        let vote_state = VoteStateV3::new(
            &VoteInit {
                node_pubkey: Pubkey::new_unique(),
                authorized_voter: Pubkey::new_unique(),
                authorized_withdrawer: Pubkey::new_unique(),
                commission: 0,
            },
            &Clock::default(),
        );
        let account_state = VoteStateVersions::new_v3(vote_state.clone());
        VoteStateV3::serialize(&account_state, &mut vote_account_data).unwrap();
        assert!(VoteStateVersions::is_correct_size_and_initialized(
            &vote_account_data
        ));

        // Check old VoteStateV3 that hasn't been upgraded to newest version yet
        let old_vote_state = VoteState1_14_11::from(vote_state);
        let account_state = VoteStateVersions::V1_14_11(Box::new(old_vote_state));
        let mut vote_account_data = vec![0; VoteState1_14_11::size_of()];
        VoteStateV3::serialize(&account_state, &mut vote_account_data).unwrap();
        assert!(VoteStateVersions::is_correct_size_and_initialized(
            &vote_account_data
        ));
    }

    #[test]
    fn test_minimum_balance() {
        let rent = solana_rent::Rent::default();
        let minimum_balance = rent.minimum_balance(VoteStateV3::size_of());
        // golden, may need updating when vote_state grows
        assert!(minimum_balance as f64 / 10f64.powf(9.0) < 0.04)
    }

    #[test]
    fn test_serde_compact_vote_state_update() {
        let mut rng = rand::thread_rng();
        for _ in 0..5000 {
            run_serde_compact_vote_state_update(&mut rng);
        }
    }

    fn run_serde_compact_vote_state_update<R: Rng>(rng: &mut R) {
        let lockouts: VecDeque<_> = std::iter::repeat_with(|| {
            let slot = 149_303_885_u64.saturating_add(rng.gen_range(0..10_000));
            let confirmation_count = rng.gen_range(0..33);
            Lockout::new_with_confirmation_count(slot, confirmation_count)
        })
        .take(32)
        .sorted_by_key(|lockout| lockout.slot())
        .collect();
        let root = rng.gen_ratio(1, 2).then(|| {
            lockouts[0]
                .slot()
                .checked_sub(rng.gen_range(0..1_000))
                .expect("All slots should be greater than 1_000")
        });
        let timestamp = rng.gen_ratio(1, 2).then(|| rng.gen());
        let hash = Hash::from(rng.gen::<[u8; 32]>());
        let vote_state_update = VoteStateUpdate {
            lockouts,
            root,
            hash,
            timestamp,
        };
        #[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
        enum VoteInstruction {
            #[serde(with = "serde_compact_vote_state_update")]
            UpdateVoteState(VoteStateUpdate),
            UpdateVoteStateSwitch(
                #[serde(with = "serde_compact_vote_state_update")] VoteStateUpdate,
                Hash,
            ),
        }
        let vote = VoteInstruction::UpdateVoteState(vote_state_update.clone());
        let bytes = bincode::serialize(&vote).unwrap();
        assert_eq!(vote, bincode::deserialize(&bytes).unwrap());
        let hash = Hash::from(rng.gen::<[u8; 32]>());
        let vote = VoteInstruction::UpdateVoteStateSwitch(vote_state_update, hash);
        let bytes = bincode::serialize(&vote).unwrap();
        assert_eq!(vote, bincode::deserialize(&bytes).unwrap());
    }

    #[test]
    fn test_circbuf_oob() {
        // Craft an invalid CircBuf with out-of-bounds index
        let data: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
        let circ_buf: CircBuf<()> = bincode::deserialize(data).unwrap();
        assert_eq!(circ_buf.last(), None);
    }

    #[test]
    fn test_vote_state_v4_bls_pubkey_compressed() {
        let vote_pubkey = Pubkey::new_unique();

        let run_test = |start, expected| {
            let versioned = VoteStateVersions::new_v4(start);
            let serialized = bincode::serialize(&versioned).unwrap();
            let deserialized = VoteStateV4::deserialize(&serialized, &vote_pubkey).unwrap();
            assert_eq!(deserialized.bls_pubkey_compressed, expected);
        };

        // First try `None`.
        let vote_state_none = VoteStateV4::default();
        assert_eq!(vote_state_none.bls_pubkey_compressed, None);
        run_test(vote_state_none, None);

        // Now try `Some`.
        let test_bls_key = [42u8; BLS_PUBLIC_KEY_COMPRESSED_SIZE];
        let vote_state_some = VoteStateV4 {
            bls_pubkey_compressed: Some(test_bls_key),
            ..VoteStateV4::default()
        };
        assert_eq!(vote_state_some.bls_pubkey_compressed, Some(test_bls_key));
        run_test(vote_state_some, Some(test_bls_key));
    }

    #[test]
    fn test_vote_state_version_conversion_bls_pubkey() {
        let vote_pubkey = Pubkey::new_unique();

        // All versions before v4 should result in `None` for BLS pubkey.
        let v0_23_5_state = VoteState0_23_5::default();
        let v0_23_5_versioned = VoteStateVersions::V0_23_5(Box::new(v0_23_5_state));

        let v1_14_11_state = VoteState1_14_11::default();
        let v1_14_11_versioned = VoteStateVersions::V1_14_11(Box::new(v1_14_11_state));

        let v3_state = VoteStateV3::default();
        let v3_versioned = VoteStateVersions::V3(Box::new(v3_state));

        for versioned in [v0_23_5_versioned, v1_14_11_versioned, v3_versioned] {
            let converted = versioned.try_convert_to_v4(&vote_pubkey).unwrap();
            assert_eq!(converted.bls_pubkey_compressed, None);
        }

        // v4 to v4 conversion should preserve the BLS pubkey.
        let test_bls_key = [128u8; BLS_PUBLIC_KEY_COMPRESSED_SIZE];
        let v4_state = VoteStateV4 {
            bls_pubkey_compressed: Some(test_bls_key),
            ..VoteStateV4::default()
        };
        let v4_versioned = VoteStateVersions::V4(Box::new(v4_state));
        let converted = v4_versioned.try_convert_to_v4(&vote_pubkey).unwrap();
        assert_eq!(converted.bls_pubkey_compressed, Some(test_bls_key));
    }
}
