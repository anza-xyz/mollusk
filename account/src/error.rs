use {solana_account::Account, solana_pubkey::Pubkey, std::fmt};

/// Why an account could not be read back into a builder.
#[derive(Debug, PartialEq)]
pub enum AccountError {
    WrongOwner { expected: Pubkey, found: Pubkey },
    InvalidData,
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongOwner { expected, found } => {
                write!(f, "account is owned by {found}, expected {expected}")
            }
            Self::InvalidData => f.write_str("account data is not valid state for this builder"),
        }
    }
}

impl std::error::Error for AccountError {}

pub(crate) fn expect_owner(account: &Account, expected: Pubkey) -> Result<(), AccountError> {
    if account.owner == expected {
        Ok(())
    } else {
        Err(AccountError::WrongOwner {
            expected,
            found: account.owner,
        })
    }
}
