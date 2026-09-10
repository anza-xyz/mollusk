use {
    crate::lamports::Lamports, solana_account::Account, solana_pubkey::Pubkey, solana_rent::Rent,
};

/// A System account.
pub struct System {
    address: Pubkey,
    lamports: Lamports,
    space: usize,
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

    /// Hold exactly `lamports`.
    pub fn lamports(mut self, lamports: u64) -> Self {
        self.lamports = Lamports::Exactly(lamports);
        self
    }

    /// Allocate `space` zeroed bytes of account data.
    pub fn space(mut self, space: usize) -> Self {
        self.space = space;
        self
    }

    /// Hold the minimum balance required for rent exemption at this account's
    /// space, under the default rent.
    ///
    /// Use [`System::rent_exempt_with`] if the test overrides
    /// `Mollusk::sysvars.rent`.
    pub fn rent_exempt(self) -> Self {
        self.rent_exempt_with(&Rent::default())
    }

    /// Hold the minimum balance required for rent exemption at this account's
    /// space, under the given `rent`.
    pub fn rent_exempt_with(mut self, rent: &Rent) -> Self {
        self.lamports = Lamports::RentExempt(rent.clone());
        self
    }
}

impl From<System> for (Pubkey, Account) {
    fn from(
        System {
            address,
            lamports,
            space,
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
