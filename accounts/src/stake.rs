//! Stake accounts.

use {
    crate::lamports::Lamports,
    solana_account::Account,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    solana_stake_interface::{
        stake_flags::StakeFlags,
        state::{Delegation, Meta, Stake as StakeState, StakeStateV2},
    },
};

/// A stake account.
///
/// Defaults to an initialized, undelegated account with no authorities,
/// funded with the rent-exempt minimum plus whatever stake is delegated.
pub struct Stake {
    address: Pubkey,
    state: StakeStateV2,
    lamports: Lamports,
}

impl Stake {
    /// An initialized stake account.
    pub fn new(address: Pubkey) -> Self {
        Self::from_state(address, StakeStateV2::Initialized(Meta::default()))
    }

    pub fn from_state(address: Pubkey, state: StakeStateV2) -> Self {
        Self {
            address,
            state,
            lamports: Lamports::RentExempt(Rent::default()),
        }
    }

    /// Set the authority permitted to delegate and deactivate the stake.
    pub fn staker(self, staker: Pubkey) -> Self {
        self.with_meta(|meta| meta.authorized.staker = staker)
    }

    /// Set the authority permitted to withdraw from the account.
    pub fn withdrawer(self, withdrawer: Pubkey) -> Self {
        self.with_meta(|meta| meta.authorized.withdrawer = withdrawer)
    }

    /// Delegate to the given vote account.
    pub fn delegated_to(self, vote_account: Pubkey) -> Self {
        self.with_delegation(|delegation| delegation.voter_pubkey = vote_account)
    }

    /// Set the active delegated stake, in lamports.
    pub fn stake(self, stake: u64) -> Self {
        self.with_delegation(|delegation| delegation.stake = stake)
    }

    /// Hold exactly `lamports`, irrespective of rent-exemption.
    pub fn lamports(mut self, lamports: u64) -> Self {
        self.lamports = Lamports::Exactly(lamports);
        self
    }

    /// Cover the rent-exempt minimum plus the delegated stake, under the
    /// default rent.
    pub fn rent_exempt(self) -> Self {
        self.rent_exempt_with(&Rent::default())
    }

    /// Cover the rent-exempt minimum under the given `rent`, plus the
    /// delegated stake.
    pub fn rent_exempt_with(mut self, rent: &Rent) -> Self {
        self.lamports = Lamports::RentExempt(rent.clone());
        self
    }

    fn with_meta(mut self, f: impl FnOnce(&mut Meta)) -> Self {
        match &mut self.state {
            StakeStateV2::Initialized(meta) | StakeStateV2::Stake(meta, _, _) => f(meta),
            StakeStateV2::Uninitialized | StakeStateV2::RewardsPool => {
                let mut meta = Meta::default();
                f(&mut meta);
                self.state = StakeStateV2::Initialized(meta);
            }
        }
        self
    }

    fn with_delegation(mut self, f: impl FnOnce(&mut Delegation)) -> Self {
        let (meta, mut stake, flags) = match self.state {
            StakeStateV2::Stake(meta, stake, flags) => (meta, stake, flags),
            StakeStateV2::Initialized(meta) => (meta, StakeState::default(), StakeFlags::empty()),
            StakeStateV2::Uninitialized | StakeStateV2::RewardsPool => {
                (Meta::default(), StakeState::default(), StakeFlags::empty())
            }
        };
        f(&mut stake.delegation);
        self.state = StakeStateV2::Stake(meta, stake, flags);
        self
    }
}

impl From<Stake> for (Pubkey, Account) {
    fn from(
        Stake {
            address,
            mut state,
            lamports,
        }: Stake,
    ) -> Self {
        let space = StakeStateV2::size_of();
        let rent_exempt_reserve = lamports.resolve(space);

        let delegated = match &mut state {
            StakeStateV2::Stake(meta, stake, _) => {
                set_reserve(meta, rent_exempt_reserve);
                stake.delegation.stake
            }
            StakeStateV2::Initialized(meta) => {
                set_reserve(meta, rent_exempt_reserve);
                0
            }
            StakeStateV2::Uninitialized | StakeStateV2::RewardsPool => 0,
        };

        let lamports = match lamports {
            Lamports::Exactly(lamports) => lamports,
            Lamports::RentExempt(_) => rent_exempt_reserve.saturating_add(delegated),
        };

        let mut data = vec![0u8; space];
        bincode::serialize_into(&mut data[..], &state).unwrap();

        (
            address,
            Account {
                lamports,
                data,
                owner: solana_sdk_ids::stake::id(),
                executable: false,
                rent_epoch: 0,
            },
        )
    }
}

#[allow(deprecated)]
fn set_reserve(meta: &mut Meta, rent_exempt_reserve: u64) {
    if meta.rent_exempt_reserve == 0 {
        meta.rent_exempt_reserve = rent_exempt_reserve;
    }
}
