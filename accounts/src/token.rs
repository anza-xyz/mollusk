//! SPL Token accounts.

use {
    crate::lamports::Lamports,
    solana_account::Account,
    solana_program_option::COption,
    solana_program_pack::Pack,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    spl_token_interface::state::{Account as TokenState, AccountState, Mint as MintState},
};

/// An SPL Token account.
///
/// Defaults to an initialized, empty account with no mint or owner, funded
/// with the rent-exempt minimum.
pub struct TokenAccount {
    address: Pubkey,
    state: TokenState,
    lamports: Lamports,
}

impl TokenAccount {
    /// An initialized token account, holding no tokens.
    pub fn new(address: Pubkey) -> Self {
        Self::from_state(
            address,
            TokenState {
                state: AccountState::Initialized,
                ..Default::default()
            },
        )
    }

    pub fn from_state(address: Pubkey, state: TokenState) -> Self {
        Self {
            address,
            state,
            lamports: Lamports::RentExempt(Rent::default()),
        }
    }

    /// Set the mint whose tokens this account holds.
    pub fn mint(mut self, mint: Pubkey) -> Self {
        self.state.mint = mint;
        self
    }

    /// Set the wallet that owns this token account.
    pub fn owner(mut self, owner: Pubkey) -> Self {
        self.state.owner = owner;
        self
    }

    /// Hold `amount` tokens.
    pub fn balance(mut self, amount: u64) -> Self {
        self.state.amount = amount;
        self
    }

    /// Freeze the account.
    pub fn frozen(mut self) -> Self {
        self.state.state = AccountState::Frozen;
        self
    }

    /// Delegate `amount` tokens to `delegate`.
    pub fn delegate(mut self, delegate: Pubkey, amount: u64) -> Self {
        self.state.delegate = COption::Some(delegate);
        self.state.delegated_amount = amount;
        self
    }

    /// Hold exactly `lamports`, rather than the rent-exempt minimum.
    pub fn lamports(mut self, lamports: u64) -> Self {
        self.lamports = Lamports::Exactly(lamports);
        self
    }

    /// Hold the rent-exempt minimum, under the default rent.
    pub fn rent_exempt(self) -> Self {
        self.rent_exempt_with(&Rent::default())
    }

    /// Hold the rent-exempt minimum under the given `rent`.
    pub fn rent_exempt_with(mut self, rent: &Rent) -> Self {
        self.lamports = Lamports::RentExempt(rent.clone());
        self
    }
}

impl From<TokenAccount> for (Pubkey, Account) {
    fn from(
        TokenAccount {
            address,
            state,
            lamports,
        }: TokenAccount,
    ) -> Self {
        (address, pack(state, lamports))
    }
}

/// An SPL Token mint.
///
/// Defaults to an initialized mint with zero supply and zero decimals, funded
/// with the rent-exempt minimum.
pub struct Mint {
    address: Pubkey,
    state: MintState,
    lamports: Lamports,
}

impl Mint {
    /// An initialized mint with no supply, no decimals, and no authorities.
    pub fn new(address: Pubkey) -> Self {
        Self::from_state(
            address,
            MintState {
                is_initialized: true,
                ..Default::default()
            },
        )
    }

    pub fn from_state(address: Pubkey, state: MintState) -> Self {
        Self {
            address,
            state,
            lamports: Lamports::RentExempt(Rent::default()),
        }
    }

    /// Set the total supply, in base units.
    pub fn supply(mut self, supply: u64) -> Self {
        self.state.supply = supply;
        self
    }

    /// Set the number of base-10 digits to the right of the decimal place.
    pub fn decimals(mut self, decimals: u8) -> Self {
        self.state.decimals = decimals;
        self
    }

    /// Set the authority permitted to mint new tokens.
    pub fn mint_authority(mut self, authority: Pubkey) -> Self {
        self.state.mint_authority = COption::Some(authority);
        self
    }

    /// Set the authority permitted to freeze token accounts.
    pub fn freeze_authority(mut self, authority: Pubkey) -> Self {
        self.state.freeze_authority = COption::Some(authority);
        self
    }

    /// Hold exactly `lamports`, rather than the rent-exempt minimum.
    pub fn lamports(mut self, lamports: u64) -> Self {
        self.lamports = Lamports::Exactly(lamports);
        self
    }

    /// Hold the rent-exempt minimum, under the default rent.
    pub fn rent_exempt(self) -> Self {
        self.rent_exempt_with(&Rent::default())
    }

    /// Hold the rent-exempt minimum under the given `rent`.
    pub fn rent_exempt_with(mut self, rent: &Rent) -> Self {
        self.lamports = Lamports::RentExempt(rent.clone());
        self
    }
}

impl From<Mint> for (Pubkey, Account) {
    fn from(
        Mint {
            address,
            state,
            lamports,
        }: Mint,
    ) -> Self {
        (address, pack(state, lamports))
    }
}

fn pack<T: Pack>(state: T, lamports: Lamports) -> Account {
    let mut data = vec![0u8; T::LEN];
    T::pack(state, &mut data).unwrap();
    Account {
        lamports: lamports.resolve(T::LEN),
        data,
        owner: spl_token_interface::id(),
        executable: false,
        rent_epoch: 0,
    }
}
