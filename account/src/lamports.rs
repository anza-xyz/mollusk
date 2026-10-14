use solana_rent::Rent;

pub(crate) enum Lamports {
    Exactly(u64),
    RentExempt(Rent),
}

impl Lamports {
    pub(crate) fn resolve(&self, space: usize) -> u64 {
        match self {
            Self::Exactly(lamports) => *lamports,
            Self::RentExempt(rent) => rent.minimum_balance(space),
        }
    }

    pub(crate) fn rent_exempt_reserve(&self, space: usize) -> u64 {
        match self {
            Self::Exactly(_) => Rent::default().minimum_balance(space),
            Self::RentExempt(rent) => rent.minimum_balance(space),
        }
    }
}
