//! Tests for the account setup helpers in `mollusk-svm-accounts`.

use {
    mollusk_svm::{result::Check, Mollusk},
    mollusk_svm_accounts::{Mint, Stake, System, TokenAccount},
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
    solana_system_program::system_processor::DEFAULT_COMPUTE_UNITS,
};

#[test]
fn test_system_transfer() {
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let base_lamports = 100_000_000u64;
    let transfer_amount = 42_000u64;

    Mollusk::default().process_and_validate_instruction(
        &solana_system_interface::instruction::transfer(&sender, &recipient, transfer_amount),
        &[
            System::new(sender).lamports(base_lamports).into(),
            System::new(recipient).lamports(base_lamports).into(),
        ],
        &[
            Check::success(),
            Check::compute_units(DEFAULT_COMPUTE_UNITS),
            Check::account(&sender)
                .lamports(base_lamports - transfer_amount)
                .build(),
            Check::account(&recipient)
                .lamports(base_lamports + transfer_amount)
                .build(),
        ],
    );
}

#[test]
fn test_rent_exemption() {
    let mollusk = Mollusk::default();

    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let space = 256usize;
    let rent_exempt_minimum = mollusk.sysvars.rent.minimum_balance(space);
    let transfer_amount = 42_000u64;

    mollusk.process_and_validate_instruction(
        &solana_system_interface::instruction::transfer(&sender, &recipient, transfer_amount),
        &[
            System::new(sender).lamports(100_000_000).into(),
            System::new(recipient).space(space).rent_exempt().into(),
        ],
        &[
            Check::success(),
            Check::account(&recipient)
                // TODO: Then here, we could add a new check API for `.rent_exempt_plus(lamports)`
                .lamports(rent_exempt_minimum + transfer_amount)
                .space(space)
                .owner(&solana_sdk_ids::system_program::id())
                .build(),
        ],
    );
}

#[test]
fn test_showcase() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");
    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "test_program_primary");

    let alice = Pubkey::new_unique();
    let bob = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let alice_token_account = Pubkey::new_unique();
    let alice_stake_account = Pubkey::new_unique();
    let vote_account = Pubkey::new_unique();

    mollusk.process_and_validate_instruction(
        &Instruction::new_with_bytes(program_id, &[0], Vec::new()),
        &[
            // Alice has 10 SOL. Bob has an empty wallet.
            System::new(alice).lamports(10_000_000_000).into(),
            System::new(bob).into(),
            // A mint with 9 decimals, one billion supply, and Alice holding the
            // mint authority.
            Mint::new(mint)
                .decimals(9)
                .supply(1_000_000_000)
                .mint_authority(alice)
                .rent_exempt()
                .into(),
            // Alice holds 1,000 tokens.
            TokenAccount::new(alice_token_account)
                .mint(mint)
                .owner(alice)
                .balance(1_000)
                .rent_exempt()
                .into(),
            // and has 5 SOL of active stake delegated to a vote account.
            Stake::new(alice_stake_account)
                .staker(alice)
                .withdrawer(alice)
                .delegated_to(vote_account)
                .stake(5_000_000_000)
                .rent_exempt()
                .into(),
        ],
        &[Check::success()],
    );
}
