use {
    num_traits::{FromPrimitive, ToPrimitive},
    solana_program_error::{ProgramError, ToStr},
};

// Use strum when testing to ensure our FromPrimitive
// impl is exhaustive
#[cfg_attr(test, derive(strum_macros::FromRepr, strum_macros::EnumIter))]
#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Deserialize, serde_derive::Serialize)
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemError {
    /// An account with the same address already exists.
    AccountAlreadyInUse,
    /// Account does not have enough SOL to perform the operation.
    ResultWithNegativeLamports,
    /// Cannot assign account to this program id.
    InvalidProgramId,
    /// Cannot allocate account data of this length.
    InvalidAccountDataLength,
    /// Length of requested seed is too long.
    MaxSeedLengthExceeded,
    /// Provided address does not match addressed derived from seed.
    AddressWithSeedMismatch,
    /// Advancing stored nonce requires a populated RecentBlockhashes sysvar.
    NonceNoRecentBlockhashes,
    /// Stored nonce is still in recent_blockhashes.
    NonceBlockhashNotExpired,
    /// Specified nonce does not match stored nonce.
    NonceUnexpectedBlockhashValue,
}

impl FromPrimitive for SystemError {
    #[inline]
    fn from_i64(n: i64) -> Option<Self> {
        if n == Self::AccountAlreadyInUse as i64 {
            Some(Self::AccountAlreadyInUse)
        } else if n == Self::ResultWithNegativeLamports as i64 {
            Some(Self::ResultWithNegativeLamports)
        } else if n == Self::InvalidProgramId as i64 {
            Some(Self::InvalidProgramId)
        } else if n == Self::InvalidAccountDataLength as i64 {
            Some(Self::InvalidAccountDataLength)
        } else if n == Self::MaxSeedLengthExceeded as i64 {
            Some(Self::MaxSeedLengthExceeded)
        } else if n == Self::AddressWithSeedMismatch as i64 {
            Some(Self::AddressWithSeedMismatch)
        } else if n == Self::NonceNoRecentBlockhashes as i64 {
            Some(Self::NonceNoRecentBlockhashes)
        } else if n == Self::NonceBlockhashNotExpired as i64 {
            Some(Self::NonceBlockhashNotExpired)
        } else if n == Self::NonceUnexpectedBlockhashValue as i64 {
            Some(Self::NonceUnexpectedBlockhashValue)
        } else {
            None
        }
    }
    #[inline]
    fn from_u64(n: u64) -> Option<Self> {
        Self::from_i64(n as i64)
    }
}

impl ToPrimitive for SystemError {
    #[inline]
    fn to_i64(&self) -> Option<i64> {
        Some(match *self {
            Self::AccountAlreadyInUse => Self::AccountAlreadyInUse as i64,
            Self::ResultWithNegativeLamports => Self::ResultWithNegativeLamports as i64,
            Self::InvalidProgramId => Self::InvalidProgramId as i64,
            Self::InvalidAccountDataLength => Self::InvalidAccountDataLength as i64,
            Self::MaxSeedLengthExceeded => Self::MaxSeedLengthExceeded as i64,
            Self::AddressWithSeedMismatch => Self::AddressWithSeedMismatch as i64,
            Self::NonceNoRecentBlockhashes => Self::NonceNoRecentBlockhashes as i64,
            Self::NonceBlockhashNotExpired => Self::NonceBlockhashNotExpired as i64,
            Self::NonceUnexpectedBlockhashValue => Self::NonceUnexpectedBlockhashValue as i64,
        })
    }
    #[inline]
    fn to_u64(&self) -> Option<u64> {
        self.to_i64().map(|x| x as u64)
    }
}

impl core::error::Error for SystemError {}

impl ToStr for SystemError {
    fn to_str(&self) -> &'static str {
        match self {
            SystemError::AccountAlreadyInUse => "an account with the same address already exists",
            SystemError::ResultWithNegativeLamports => {
                "account does not have enough SOL to perform the operation"
            }
            SystemError::InvalidProgramId => "cannot assign account to this program id",
            SystemError::InvalidAccountDataLength => "cannot allocate account data of this length",
            SystemError::MaxSeedLengthExceeded => "length of requested seed is too long",
            SystemError::AddressWithSeedMismatch => {
                "provided address does not match addressed derived from seed"
            }
            SystemError::NonceNoRecentBlockhashes => {
                "advancing stored nonce requires a populated RecentBlockhashes sysvar"
            }
            SystemError::NonceBlockhashNotExpired => "stored nonce is still in recent_blockhashes",
            SystemError::NonceUnexpectedBlockhashValue => {
                "specified nonce does not match stored nonce"
            }
        }
    }
}

impl core::fmt::Display for SystemError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl From<SystemError> for ProgramError {
    fn from(e: SystemError) -> Self {
        Self::Custom(e as u32)
    }
}

impl TryFrom<u32> for SystemError {
    type Error = ProgramError;
    fn try_from(error: u32) -> Result<Self, Self::Error> {
        match error {
            0 => Ok(SystemError::AccountAlreadyInUse),
            1 => Ok(SystemError::ResultWithNegativeLamports),
            2 => Ok(SystemError::InvalidProgramId),
            3 => Ok(SystemError::InvalidAccountDataLength),
            4 => Ok(SystemError::MaxSeedLengthExceeded),
            5 => Ok(SystemError::AddressWithSeedMismatch),
            6 => Ok(SystemError::NonceNoRecentBlockhashes),
            7 => Ok(SystemError::NonceBlockhashNotExpired),
            8 => Ok(SystemError::NonceUnexpectedBlockhashValue),
            _ => Err(ProgramError::InvalidArgument),
        }
    }
}

impl From<u64> for SystemError {
    fn from(error: u64) -> Self {
        SystemError::from_u64(error).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use {super::SystemError, num_traits::FromPrimitive, strum::IntoEnumIterator};

    #[test]
    fn test_system_error_from_primitive_exhaustive() {
        for variant in SystemError::iter() {
            let variant_i64 = variant.clone() as i64;
            assert_eq!(
                SystemError::from_repr(variant_i64 as usize),
                SystemError::from_i64(variant_i64)
            );
            assert_eq!(SystemError::from(variant_i64 as u64), variant);
        }
    }
}
