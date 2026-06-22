//! Last updated at mainnet-beta slot height: 347196212
//!
//! SVM program helpers for SPL Token-2022. Base program helpers are always
//! available; extension-aware builders are behind the `extensions` feature.
//! Re-exported as `mollusk_svm_programs_token::token2022` for compatibility.

#[cfg(feature = "extensions")]
mod extensions;
#[cfg(feature = "extensions")]
pub use extensions::*;
use {
    mollusk_svm::Mollusk, solana_account::Account, solana_program_pack::Pack,
    solana_pubkey::Pubkey, solana_rent::Rent,
};

pub const ID: Pubkey = solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

pub const ELF: &[u8] = include_bytes!("elf/token_2022.so");

pub fn add_program(mollusk: &mut Mollusk) {
    // Loader v3
    mollusk.add_program_with_loader_and_elf(
        &ID,
        &mollusk_svm::program::loader_keys::LOADER_V3,
        ELF,
    );
}

pub fn account() -> Account {
    // Loader v3
    mollusk_svm::program::create_program_account_loader_v3(&ID)
}

/// Get the key and account for the SPL Token-2022 program.
pub fn keyed_account() -> (Pubkey, Account) {
    (ID, account())
}

// Rent-exempt account owned by the Token-2022 program.
pub(crate) fn rent_exempt_account(data: Vec<u8>) -> Account {
    Account {
        lamports: Rent::default().minimum_balance(data.len()),
        data,
        owner: ID,
        executable: false,
        rent_epoch: 0,
    }
}

/// Create a Mint Account
pub fn create_account_for_mint(mint_data: spl_token_interface::state::Mint) -> Account {
    use spl_token_interface::state::Mint;
    let mut data = vec![0u8; Mint::LEN];
    Mint::pack(mint_data, &mut data).unwrap();
    rent_exempt_account(data)
}

/// Create a Token Account
pub fn create_account_for_token_account(
    token_account_data: spl_token_interface::state::Account,
) -> Account {
    use spl_token_interface::state::Account as TokenAccount;
    let mut data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(token_account_data, &mut data).unwrap();
    rent_exempt_account(data)
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        solana_program_option::COption,
        spl_token_interface::state::{Account as TokenAccount, AccountState, Mint},
    };

    #[test]
    fn base_account_layouts() {
        let mint = create_account_for_mint(Mint {
            mint_authority: COption::Some(Pubkey::new_unique()),
            supply: 0,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::None,
        });
        assert_eq!(mint.data.len(), Mint::LEN);
        assert_eq!(mint.owner, ID);

        let token = create_account_for_token_account(TokenAccount {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: 0,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        });
        assert_eq!(token.data.len(), TokenAccount::LEN);
        assert_eq!(token.owner, ID);
    }
}
