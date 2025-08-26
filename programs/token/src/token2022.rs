use {
    mollusk_svm::Mollusk,
    solana_account::Account,
    solana_program_pack::Pack,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    spl_token_interface::state::{Account as TokenAccount, Mint},
};

pub const ID: Pubkey = solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

pub const ELF: &[u8] = include_bytes!("elf/token_2022.so");

pub fn add_program(mollusk: &mut Mollusk) {
    // Loader v3
    mollusk.add_program_with_elf_and_loader(
        &ID,
        ELF,
        &mollusk_svm::program::loader_keys::LOADER_V3,
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

/// Create a Mint Account
pub fn create_account_for_mint(mint_data: Mint) -> Account {
    let mut data = vec![0u8; Mint::LEN];
    Mint::pack(mint_data, &mut data).unwrap();

    Account {
        lamports: Rent::default().minimum_balance(Mint::LEN),
        data,
        owner: ID,
        executable: false,
        rent_epoch: 0,
    }
}

/// Create a Token Account
pub fn create_account_for_token_account(token_account_data: TokenAccount) -> Account {
    let mut data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(token_account_data, &mut data).unwrap();

    Account {
        lamports: Rent::default().minimum_balance(TokenAccount::LEN),
        data,
        owner: ID,
        executable: false,
        rent_epoch: 0,
    }
}
