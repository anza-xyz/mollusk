use {
    mollusk_svm::Mollusk, solana_account::Account, solana_program_pack::Pack,
    solana_pubkey::Pubkey, solana_rent::Rent,
    spl_associated_token_account_interface::address::get_associated_token_address_with_program_id,
    spl_token_interface::state::Account as TokenAccount,
};

pub const ID: Pubkey = solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const TOKEN_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const TOKEN_2022_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

pub const ELF: &[u8] = include_bytes!("elf/associated_token.so");

pub fn add_program(mollusk: &mut Mollusk) {
    // Loader v2
    mollusk.add_program_with_elf_and_loader(
        &ID,
        ELF,
        &mollusk_svm::program::loader_keys::LOADER_V2,
    );
}

pub fn account() -> Account {
    // Loader v2
    mollusk_svm::program::create_program_account_loader_v2(ELF)
}

/// Get the key and account for the SPL Associated Token program.
pub fn keyed_account() -> (Pubkey, Account) {
    (ID, account())
}

/// Create an Associated Token Account
pub fn create_account_for_associated_token_account(
    token_account_data: TokenAccount,
) -> (Pubkey, Account) {
    let associated_token_address = get_associated_token_address_with_program_id(
        &token_account_data.owner,
        &token_account_data.mint,
        &TOKEN_PROGRAM_ID,
    );

    let mut data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(token_account_data, &mut data).unwrap();

    let account = Account {
        lamports: Rent::default().minimum_balance(TokenAccount::LEN),
        data,
        owner: TOKEN_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };

    (associated_token_address, account)
}

/// Create an Associated Token Account for the Token2022 program
pub fn create_account_for_associated_token_2022_account(
    token_account_data: TokenAccount,
) -> (Pubkey, Account) {
    let associated_token_address = get_associated_token_address_with_program_id(
        &token_account_data.owner,
        &token_account_data.mint,
        &TOKEN_2022_PROGRAM_ID,
    );

    // space for immutable owner extension and account type
    const EXTENDED_ACCOUNT_LEN: usize = TokenAccount::LEN + 5;
    let mut data = vec![0u8; EXTENDED_ACCOUNT_LEN];
    TokenAccount::pack(token_account_data, &mut data).unwrap();
    data[TokenAccount::LEN] = 2; // AccountType::Account
    data[TokenAccount::LEN + 1] = 7; // ExtensionType::ImmutableOwner

    let account = Account {
        lamports: Rent::default().minimum_balance(EXTENDED_ACCOUNT_LEN),
        data,
        owner: TOKEN_2022_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };

    (associated_token_address, account)
}
