//! Account setup for arbitrary program state.
//!
//! ```
//! use {
//!     mollusk_svm_account::{account, AccountState},
//!     solana_pubkey::Pubkey,
//! };
//!
//! const MY_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1; 32]);
//!
//! #[derive(wincode::SchemaWrite)]
//! struct Counter {
//!     count: u64,
//! }
//!
//! impl AccountState for Counter {
//!     fn encode(&self) -> Vec<u8> {
//!         wincode::serialize(self).unwrap()
//!     }
//!
//!     fn owner() -> Pubkey {
//!         MY_PROGRAM_ID
//!     }
//! }
//!
//! let account = account(&Counter { count: 42 });
//! assert_eq!(account.owner, MY_PROGRAM_ID);
//! ```

use {solana_account::Account, solana_pubkey::Pubkey, solana_rent::Rent};

/// State held by an account, and the program that owns it.
pub trait AccountState {
    /// Encode the state into account data.
    fn encode(&self) -> Vec<u8>;

    /// The program that owns accounts holding this state.
    fn owner() -> Pubkey;

    /// Bytes to allocate, given the length of the encoded state.
    ///
    /// Defaults to the encoded length. Override it for state whose account is
    /// a fixed size, or is padded beyond what the encoding produces.
    fn space(&self, encoded_len: usize) -> usize {
        encoded_len
    }
}

/// Build a rent-exempt account holding `state`, owned by its program.
pub fn account<S: AccountState>(state: &S) -> Account {
    let encoded = state.encode();
    let space = state.space(encoded.len());

    let mut account = Account::new(Rent::default().minimum_balance(space), space, &S::owner());
    let len = encoded.len().min(space);
    account.data[..len].copy_from_slice(&encoded[..len]);
    account
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROGRAM_ID: Pubkey = Pubkey::new_from_array([7; 32]);

    #[derive(Debug, PartialEq, wincode::SchemaRead, wincode::SchemaWrite)]
    struct Counter {
        count: u64,
        authority: [u8; 32],
    }

    impl AccountState for Counter {
        fn encode(&self) -> Vec<u8> {
            wincode::serialize(self).unwrap()
        }

        fn owner() -> Pubkey {
            PROGRAM_ID
        }
    }

    #[derive(wincode::SchemaWrite)]
    struct Padded {
        count: u64,
    }

    impl AccountState for Padded {
        fn encode(&self) -> Vec<u8> {
            wincode::serialize(self).unwrap()
        }

        fn owner() -> Pubkey {
            PROGRAM_ID
        }

        fn space(&self, _encoded_len: usize) -> usize {
            128
        }
    }

    #[test]
    fn test_state_round_trips() {
        let state = Counter {
            count: 42,
            authority: [9; 32],
        };
        let account = account(&state);

        assert_eq!(account.owner, PROGRAM_ID);
        assert_eq!(
            wincode::deserialize::<Counter>(&account.data).unwrap(),
            state
        );
    }

    #[test]
    fn test_account_is_rent_exempt() {
        let account = account(&Counter {
            count: 0,
            authority: [0; 32],
        });

        assert_eq!(
            account.lamports,
            Rent::default().minimum_balance(account.data.len())
        );
    }

    #[test]
    fn test_space_defaults_to_encoded_len() {
        let state = Counter {
            count: 1,
            authority: [0; 32],
        };

        assert_eq!(
            account(&state).data.len(),
            wincode::serialize(&state).unwrap().len()
        );
    }

    #[test]
    fn test_space_override_pads_encoded_state() {
        let account = account(&Padded { count: 1 });

        assert_eq!(account.data.len(), 128);
        assert_eq!(account.lamports, Rent::default().minimum_balance(128));
    }
}
