use mollusk_svm::Mollusk;
use solana_account::Account;
use solana_pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token::{
    solana_program::program_pack::Pack,
    state::{Account as TokenAccount, AccountState, Mint},
};

use crate::token;

/// Note: Support for Token2022 state with extensions is not yet implemented

// Create a Keyed Account for a Mint with default data
#[allow(dead_code)]
pub fn keyed_account_for_mint_default(
    mollusk: &Mollusk,
    authority: &Pubkey,
    decimals: u8,
    pubkey: Option<Pubkey>,
    token_program: Option<Pubkey>,
) -> (Pubkey, Account) {
    let mint_data = Mint {
        mint_authority: Some(*authority).into(),
        supply: 0,
        decimals,
        is_initialized: true,
        freeze_authority: None.into(),
    };

    let mut data = vec![0u8; Mint::LEN];
    Mint::pack(mint_data, &mut data).unwrap();

    let account = Account {
        lamports: mollusk.sysvars.rent.minimum_balance(Mint::LEN),
        data,
        owner: token_program.unwrap_or(token::ID),
        executable: false,
        rent_epoch: 0,
    };

    (pubkey.unwrap_or(Pubkey::new_unique()), account)
}

// Create a Keyed Account for a Mint
#[allow(dead_code)]
pub fn keyed_account_for_mint(
    mollusk: &Mollusk,
    authority: &Pubkey,
    supply: u64,
    decimals: u8,
    freeze_authority: Option<Pubkey>,
    pubkey: Option<Pubkey>,
    token_program: Option<Pubkey>,
) -> (Pubkey, Account) {
    let mint_data = Mint {
        mint_authority: Some(*authority).into(),
        supply,
        decimals,
        is_initialized: true,
        freeze_authority: freeze_authority.into(),
    };

    let mut data = vec![0u8; Mint::LEN];
    Mint::pack(mint_data, &mut data).unwrap();

    let account = Account {
        lamports: mollusk.sysvars.rent.minimum_balance(Mint::LEN),
        data,
        owner: token_program.unwrap_or(token::ID),
        executable: false,
        rent_epoch: 0,
    };

    (pubkey.unwrap_or(Pubkey::new_unique()), account)
}

// Create a Keyed Account for a Token Account with default data
#[allow(dead_code)]
pub fn keyed_account_for_token_account_default(
    mollusk: &Mollusk,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
    pubkey: Option<Pubkey>,
    token_program: Option<Pubkey>,
) -> (Pubkey, Account) {
    let account_data = TokenAccount {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: None.into(),
        state: AccountState::Initialized,
        is_native: None.into(),
        delegated_amount: 0,
        close_authority: None.into(),
    };

    let mut data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(account_data, &mut data).unwrap();

    let account = Account {
        lamports: mollusk.sysvars.rent.minimum_balance(TokenAccount::LEN),
        data,
        owner: token_program.unwrap_or(token::ID),
        executable: false,
        rent_epoch: 0,
    };

    (pubkey.unwrap_or(Pubkey::new_unique()), account)
}

// Create a Keyed Account for a Token Account
#[allow(dead_code)]
pub fn keyed_account_for_token_account(
    mollusk: &Mollusk,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
    delegate: Option<Pubkey>,
    state: AccountState,
    is_native: Option<u64>,
    delegated_amount: u64,
    close_authority: Option<Pubkey>,
    pubkey: Option<Pubkey>,
    token_program: Option<Pubkey>,
) -> (Pubkey, Account) {
    let account_data = TokenAccount {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: delegate.into(),
        state,
        is_native: is_native.into(),
        delegated_amount,
        close_authority: close_authority.into(),
    };

    let mut data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(account_data, &mut data).unwrap();

    let account = Account {
        lamports: mollusk.sysvars.rent.minimum_balance(TokenAccount::LEN),
        data,
        owner: token_program.unwrap_or(token::ID),
        executable: false,
        rent_epoch: 0,
    };

    (pubkey.unwrap_or(Pubkey::new_unique()), account)
}

// Create a Keyed Account for an Associated Token Account with default data
#[allow(dead_code)]
pub fn keyed_account_for_associated_token_account_default(
    mollusk: &Mollusk,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
    token_program: Option<Pubkey>,
) -> (Pubkey, Account) {
    let associciated_token_address = get_associated_token_address_with_program_id(
        owner,
        mint,
        &token_program.unwrap_or(token::ID),
    );

    keyed_account_for_token_account_default(
        mollusk,
        mint,
        owner,
        amount,
        Some(associciated_token_address),
        Some(token_program.unwrap_or(token::ID)),
    )
}

// Create a Keyed Account for an Associated Token Account
#[allow(dead_code)]
pub fn keyed_account_for_associated_token_account(
    mollusk: &Mollusk,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
    delegate: Option<Pubkey>,
    state: AccountState,
    is_native: Option<u64>,
    delegated_amount: u64,
    close_authority: Option<Pubkey>,
    token_program: Option<Pubkey>,
) -> (Pubkey, Account) {
    let associciated_token_address = get_associated_token_address_with_program_id(
        owner,
        mint,
        &token_program.unwrap_or(token::ID),
    );

    keyed_account_for_token_account(
        mollusk,
        mint,
        owner,
        amount,
        delegate,
        state,
        is_native,
        delegated_amount,
        close_authority,
        Some(associciated_token_address),
        Some(token_program.unwrap_or(token::ID)),
    )
}
