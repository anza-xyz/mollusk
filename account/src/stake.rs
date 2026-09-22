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
pub struct Stake {
    address: Pubkey,
    state: StakeStateV2,
    lamports: Lamports,
}

impl Stake {
    /// An initialized, undelegated stake account at `address`, with no
    /// authorities, holding the rent-exempt minimum.
    pub fn initialized(address: Pubkey) -> Self {
        Self::new(address, StakeStateV2::Initialized(Meta::default()))
    }

    pub fn new(address: Pubkey, state: StakeStateV2) -> Self {
        Self {
            address,
            state,
            lamports: Lamports::RentExempt(Rent::default()),
        }
    }

    pub fn staker(self, staker: Pubkey) -> Self {
        self.with_meta(|meta| meta.authorized.staker = staker)
    }

    pub fn withdrawer(self, withdrawer: Pubkey) -> Self {
        self.with_meta(|meta| meta.authorized.withdrawer = withdrawer)
    }

    pub fn delegated_to(self, vote_account: Pubkey) -> Self {
        self.with_delegation(|delegation| delegation.voter_pubkey = vote_account)
    }

    pub fn stake(self, stake: u64) -> Self {
        self.with_delegation(|delegation| delegation.stake = stake)
    }

    pub fn lamports(mut self, lamports: u64) -> Self {
        self.lamports = Lamports::Exactly(lamports);
        self
    }

    pub fn rent_exempt(self) -> Self {
        self.rent_exempt_with(&Rent::default())
    }

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
        let rent_exempt_reserve = lamports.resolve(StakeStateV2::size_of());

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

        let mut account = crate::account(&state);
        account.lamports = match lamports {
            Lamports::Exactly(lamports) => lamports,
            Lamports::RentExempt(_) => rent_exempt_reserve.saturating_add(delegated),
        };

        (address, account)
    }
}

#[allow(deprecated)]
fn set_reserve(meta: &mut Meta, rent_exempt_reserve: u64) {
    if meta.rent_exempt_reserve == 0 {
        meta.rent_exempt_reserve = rent_exempt_reserve;
    }
}

impl crate::AccountState for StakeStateV2 {
    fn encode(&self) -> Vec<u8> {
        wincode::serialize(self).expect("wincode encoding failed")
    }

    fn owner() -> Pubkey {
        solana_sdk_ids::stake::id()
    }

    fn space(&self, _encoded_len: usize) -> usize {
        StakeStateV2::size_of()
    }
}
