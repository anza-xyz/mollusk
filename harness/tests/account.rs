//! Tests for the account setup helpers in `mollusk-svm-account`.

use {
    mollusk_svm::{result::Check, Mollusk},
    mollusk_svm_account::{account, AccountError, AccountState, Mint, Stake, System, TokenAccount},
    solana_account::Account,
    solana_instruction::{AccountMeta, Instruction},
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
            Mint::initialized(mint)
                .decimals(9)
                .supply(1_000_000_000)
                .mint_authority(alice)
                .rent_exempt()
                .into(),
            // Alice holds 1,000 tokens.
            TokenAccount::initialized(alice_token_account)
                .mint(mint)
                .owner(alice)
                .balance(1_000)
                .rent_exempt()
                .into(),
            // and has 5 SOL of active stake delegated to a vote account.
            Stake::initialized(alice_stake_account)
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

const CUSTOM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([3; 32]);

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
        CUSTOM_PROGRAM_ID
    }
}

#[test]
fn test_custom_state() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");
    let mollusk = Mollusk::new(&CUSTOM_PROGRAM_ID, "test_program_primary");

    let counter = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let initial = Counter {
        count: 0,
        authority: [0; 32],
    };
    let updated = Counter {
        count: 42,
        authority: authority.to_bytes(),
    };

    let mut instruction_data = vec![1];
    instruction_data.extend_from_slice(&updated.encode());

    let result = mollusk.process_and_validate_instruction(
        &Instruction::new_with_bytes(
            CUSTOM_PROGRAM_ID,
            &instruction_data,
            vec![AccountMeta::new(counter, true)],
        ),
        &[(counter, account(&initial))],
        &[
            Check::success(),
            Check::account(&counter)
                .owner(&CUSTOM_PROGRAM_ID)
                .space(initial.encode().len())
                .build(),
        ],
    );

    assert_eq!(
        wincode::deserialize::<Counter>(&result.get_account(&counter).unwrap().data).unwrap(),
        updated
    );
}

#[test]
fn test_round_trip() {
    let alice = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let vote = Pubkey::new_unique();

    let system: (Pubkey, Account) = System::new(alice).lamports(10_000_000_000).space(64).into();
    assert_eq!(
        <(Pubkey, Account)>::from(System::try_from(system.clone()).unwrap()),
        system
    );

    let stake: (Pubkey, Account) = Stake::initialized(Pubkey::new_unique())
        .staker(alice)
        .withdrawer(alice)
        .delegated_to(vote)
        .stake(5_000_000_000)
        .into();
    assert_eq!(
        <(Pubkey, Account)>::from(Stake::try_from(stake.clone()).unwrap()),
        stake
    );

    let mint_account: (Pubkey, Account) = Mint::initialized(mint)
        .decimals(9)
        .supply(1_000_000_000)
        .mint_authority(alice)
        .into();
    assert_eq!(
        <(Pubkey, Account)>::from(Mint::try_from(mint_account.clone()).unwrap()),
        mint_account
    );

    let token_account: (Pubkey, Account) = TokenAccount::initialized(Pubkey::new_unique())
        .mint(mint)
        .owner(alice)
        .balance(1_000)
        .into();
    assert_eq!(
        <(Pubkey, Account)>::from(TokenAccount::try_from(token_account.clone()).unwrap()),
        token_account
    );
}

#[test]
fn test_round_trip_rejects_wrong_owner() {
    let address = Pubkey::new_unique();
    let (_, account) = <(Pubkey, Account)>::from(System::new(address).lamports(1_000_000_000));

    assert!(matches!(
        Mint::try_from((address, account)),
        Err(AccountError::WrongOwner { .. })
    ));
}
