//! A trait for implementing an account store, to be used with the
/// `MolluskWithAccountStore`.
use {solana_account::Account, solana_pubkey::Pubkey};

/// A trait for implementing an account store, to be used with the
/// `MolluskWithAccountStore`.
pub trait AccountStore {
    /// Returns the default account to be used when an account is not found.
    fn default_account(&self, _pubkey: &Pubkey) -> Account {
        Account::default()
    }

    /// Get an account at the given public key.
    fn get_account(&self, pubkey: &Pubkey) -> Option<Account>;

    /// Store an account at the given public key.
    fn store_account(&mut self, pubkey: Pubkey, account: Account);
}
