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
pub struct TokenAccount {
    address: Pubkey,
    state: TokenState,
    lamports: Lamports,
}

impl TokenAccount {
    /// An initialized token account at `address`, with no mint or owner,
    /// holding no tokens and the rent-exempt minimum.
    pub fn initialized(address: Pubkey) -> Self {
        Self::new(
            address,
            TokenState {
                state: AccountState::Initialized,
                ..Default::default()
            },
        )
    }

    pub fn new(address: Pubkey, state: TokenState) -> Self {
        Self {
            address,
            state,
            lamports: Lamports::RentExempt(Rent::default()),
        }
    }

    pub fn mint(mut self, mint: Pubkey) -> Self {
        self.state.mint = mint;
        self
    }

    pub fn owner(mut self, owner: Pubkey) -> Self {
        self.state.owner = owner;
        self
    }

    pub fn balance(mut self, amount: u64) -> Self {
        self.state.amount = amount;
        self
    }

    pub fn frozen(mut self) -> Self {
        self.state.state = AccountState::Frozen;
        self
    }

    pub fn delegate(mut self, delegate: Pubkey, amount: u64) -> Self {
        self.state.delegate = COption::Some(delegate);
        self.state.delegated_amount = amount;
        self
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
}

impl From<TokenAccount> for (Pubkey, Account) {
    fn from(
        TokenAccount {
            address,
            state,
            lamports,
        }: TokenAccount,
    ) -> Self {
        (address, fund(crate::account(&state), lamports))
    }
}

/// An SPL Token mint.
pub struct Mint {
    address: Pubkey,
    state: MintState,
    lamports: Lamports,
}

impl Mint {
    /// An initialized mint at `address`, with no supply, no decimals and no
    /// authorities, holding the rent-exempt minimum.
    pub fn initialized(address: Pubkey) -> Self {
        Self::new(
            address,
            MintState {
                is_initialized: true,
                ..Default::default()
            },
        )
    }

    pub fn new(address: Pubkey, state: MintState) -> Self {
        Self {
            address,
            state,
            lamports: Lamports::RentExempt(Rent::default()),
        }
    }

    pub fn supply(mut self, supply: u64) -> Self {
        self.state.supply = supply;
        self
    }

    pub fn decimals(mut self, decimals: u8) -> Self {
        self.state.decimals = decimals;
        self
    }

    pub fn mint_authority(mut self, authority: Pubkey) -> Self {
        self.state.mint_authority = COption::Some(authority);
        self
    }

    pub fn freeze_authority(mut self, authority: Pubkey) -> Self {
        self.state.freeze_authority = COption::Some(authority);
        self
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
}

impl From<Mint> for (Pubkey, Account) {
    fn from(
        Mint {
            address,
            state,
            lamports,
        }: Mint,
    ) -> Self {
        (address, fund(crate::account(&state), lamports))
    }
}

impl crate::AccountState for TokenState {
    fn encode(&self) -> Vec<u8> {
        pack(*self)
    }

    fn owner() -> Pubkey {
        spl_token_interface::id()
    }
}

impl crate::AccountState for MintState {
    fn encode(&self) -> Vec<u8> {
        pack(*self)
    }

    fn owner() -> Pubkey {
        spl_token_interface::id()
    }
}

fn pack<T: Pack>(state: T) -> Vec<u8> {
    let mut data = vec![0u8; T::LEN];
    T::pack(state, &mut data).unwrap();
    data
}

fn fund(mut account: Account, lamports: Lamports) -> Account {
    account.lamports = lamports.resolve(account.data.len());
    account
}
