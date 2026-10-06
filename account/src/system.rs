use {
    crate::lamports::Lamports, solana_account::Account, solana_pubkey::Pubkey, solana_rent::Rent,
};

/// A System account.
pub struct System {
    address: Pubkey,
    space: usize,
    lamports: Lamports,
}

impl System {
    /// A System account at `address`, holding no lamports and no data.
    pub fn new(address: Pubkey) -> Self {
        Self {
            address,
            lamports: Lamports::Exactly(0),
            space: 0,
        }
    }

    pub fn space(mut self, space: usize) -> Self {
        self.space = space;
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

impl From<System> for (Pubkey, Account) {
    fn from(
        System {
            address,
            space,
            lamports,
        }: System,
    ) -> Self {
        (
            address,
            Account::new(
                lamports.resolve(space),
                space,
                &solana_sdk_ids::system_program::id(),
            ),
        )
    }
}
