#![cfg(any(feature = "fuzz", feature = "fuzz-fd"))]

use {
    mollusk_svm::{result::Compare, Mollusk},
    solana_account::Account,
    solana_pubkey::Pubkey,
};

const BASE_LAMPORTS: u64 = 100_000_000;

#[cfg(feature = "fuzz")]
#[test]
fn test_process_mollusk() {
    let ok_transfer_amount = 42_000;
    let too_much = BASE_LAMPORTS + 1;

    let mut mollusk = Mollusk::default();

    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let accounts = vec![
        (
            sender,
            Account::new(BASE_LAMPORTS, 0, &solana_sdk_ids::system_program::id()),
        ),
        (
            recipient,
            Account::new(BASE_LAMPORTS, 0, &solana_sdk_ids::system_program::id()),
        ),
    ];

    // First try the success case.
    let instruction =
        solana_system_interface::instruction::transfer(&sender, &recipient, ok_transfer_amount);
    let result = mollusk.process_instruction(&instruction, &accounts);

    let fixture = mollusk_svm::fuzz::mollusk::build_fixture_from_mollusk_test(
        &mollusk,
        &instruction,
        &accounts,
        &result,
    );

    mollusk.process_and_validate_fixture(&fixture);

    // Now the error case.
    let instruction = solana_system_interface::instruction::transfer(&sender, &recipient, too_much);
    let result = mollusk.process_instruction(&instruction, &accounts);

    let fixture = mollusk_svm::fuzz::mollusk::build_fixture_from_mollusk_test(
        &mollusk,
        &instruction,
        &accounts,
        &result,
    );

    mollusk.process_and_validate_fixture(&fixture);
}

#[cfg(feature = "fuzz")]
#[test]
#[should_panic(expected = "Expected: `[1, 2, 3]`")]
fn test_partially_validate_mollusk_reports_fixture_as_expected() {
    let mut mollusk = Mollusk::default();

    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let accounts = vec![
        (
            sender,
            Account::new(BASE_LAMPORTS, 0, &solana_sdk_ids::system_program::id()),
        ),
        (
            recipient,
            Account::new(BASE_LAMPORTS, 0, &solana_sdk_ids::system_program::id()),
        ),
    ];

    let instruction = solana_system_interface::instruction::transfer(&sender, &recipient, 42_000);
    let result = mollusk.process_instruction(&instruction, &accounts);

    let mut fixture = mollusk_svm::fuzz::mollusk::build_fixture_from_mollusk_test(
        &mollusk,
        &instruction,
        &accounts,
        &result,
    );
    // The transfer returns no data, so this mismatch must fail.
    fixture.output.return_data = vec![1, 2, 3];

    mollusk.process_and_partially_validate_fixture(&fixture, &[Compare::ReturnData]);
}

#[cfg(feature = "fuzz-fd")]
#[test]
fn test_process_firedancer() {
    let ok_transfer_amount = 42_000;
    let too_much = BASE_LAMPORTS + 1;

    let mut mollusk = Mollusk::default();

    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let accounts = vec![
        (
            sender,
            Account::new(BASE_LAMPORTS, 0, &solana_sdk_ids::system_program::id()),
        ),
        (
            recipient,
            Account::new(BASE_LAMPORTS, 0, &solana_sdk_ids::system_program::id()),
        ),
    ];

    // First try the success case.
    let instruction =
        solana_system_interface::instruction::transfer(&sender, &recipient, ok_transfer_amount);
    let result = mollusk.process_instruction(&instruction, &accounts);

    let fixture = mollusk_svm::fuzz::firedancer::build_fixture_from_mollusk_test(
        &mollusk,
        &instruction,
        &accounts,
        &result,
    );

    mollusk.process_and_validate_firedancer_fixture(&fixture);

    // Now the error case.
    let instruction = solana_system_interface::instruction::transfer(&sender, &recipient, too_much);
    let result = mollusk.process_instruction(&instruction, &accounts);

    let fixture = mollusk_svm::fuzz::firedancer::build_fixture_from_mollusk_test(
        &mollusk,
        &instruction,
        &accounts,
        &result,
    );

    mollusk.process_and_validate_firedancer_fixture(&fixture);
}

#[cfg(feature = "fuzz-fd")]
#[test]
fn test_process_firedancer_accounts_not_in_message_order() {
    let mut mollusk = Mollusk::default();

    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    // The compiled message puts the sender (signer) first.
    let accounts = vec![
        (
            recipient,
            Account::new(BASE_LAMPORTS, 0, &solana_sdk_ids::system_program::id()),
        ),
        (
            sender,
            Account::new(BASE_LAMPORTS, 0, &solana_sdk_ids::system_program::id()),
        ),
    ];

    let instruction = solana_system_interface::instruction::transfer(&sender, &recipient, 42_000);
    let result = mollusk.process_instruction(&instruction, &accounts);

    let fixture = mollusk_svm::fuzz::firedancer::build_fixture_from_mollusk_test(
        &mollusk,
        &instruction,
        &accounts,
        &result,
    );

    mollusk.process_and_validate_firedancer_fixture(&fixture);
}

#[cfg(feature = "fuzz-fd")]
#[test]
#[should_panic(expected = "Expected: `[1, 2, 3]`")]
fn test_partially_validate_firedancer_reports_fixture_as_expected() {
    let mut mollusk = Mollusk::default();

    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let accounts = vec![
        (
            sender,
            Account::new(BASE_LAMPORTS, 0, &solana_sdk_ids::system_program::id()),
        ),
        (
            recipient,
            Account::new(BASE_LAMPORTS, 0, &solana_sdk_ids::system_program::id()),
        ),
    ];

    let instruction = solana_system_interface::instruction::transfer(&sender, &recipient, 42_000);
    let result = mollusk.process_instruction(&instruction, &accounts);

    let mut fixture = mollusk_svm::fuzz::firedancer::build_fixture_from_mollusk_test(
        &mollusk,
        &instruction,
        &accounts,
        &result,
    );
    // The transfer returns no data, so this mismatch must fail.
    fixture.output.return_data = vec![1, 2, 3];

    mollusk.process_and_partially_validate_firedancer_fixture(&fixture, &[Compare::ReturnData]);
}
