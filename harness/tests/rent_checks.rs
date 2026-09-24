//! Tests for the rent check applied during processing.

use {
    mollusk_svm::{
        result::{types::TransactionProgramResult, Check},
        Mollusk,
    },
    solana_account::Account,
    solana_instruction::{AccountMeta, Instruction},
    solana_program_error::ProgramError,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    solana_system_interface::instruction::transfer,
    solana_transaction_error::TransactionError,
};

fn system_account(lamports: u64) -> Account {
    Account::new(lamports, 0, &solana_sdk_ids::system_program::id())
}

fn exempt_minimum() -> u64 {
    Rent::default().minimum_balance(0)
}

#[test]
#[should_panic(expected = "left in a rent state the runtime rejects")]
fn test_rent_regression_fails_by_default() {
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let sender_lamports = exempt_minimum() + 1_000;

    let instruction = transfer(&sender, &recipient, 2_000);
    let accounts = [
        (sender, system_account(sender_lamports)),
        (recipient, system_account(exempt_minimum())),
    ];

    Mollusk::default().process_and_validate_instruction(&instruction, &accounts, &[]);
}

#[test]
fn test_rent_regression_fails_no_panic() {
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let sender_lamports = exempt_minimum() + 1_000;
    let transfer_amount = 2_000;

    let instruction = transfer(&sender, &recipient, transfer_amount);
    let accounts = [
        (sender, system_account(sender_lamports)),
        (recipient, system_account(exempt_minimum())),
    ];

    let mut mollusk = Mollusk::default();
    mollusk.config.panic = false;

    let result = mollusk.process_and_validate_instruction(&instruction, &accounts, &[]);

    assert!(result.run_checks(
        &[Check::err(ProgramError::AccountNotRentExempt)],
        &mollusk.config
    ));
    assert_eq!(result.raw_result, Ok(()));
    assert_eq!(
        result.get_account(&sender).map(|account| account.lamports),
        Some(sender_lamports - transfer_amount),
    );
}

#[test]
fn test_rent_regression_in_transaction_no_panic() {
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let sender_lamports = exempt_minimum() + 1_000;

    let instruction = transfer(&sender, &recipient, 2_000);
    let accounts = [
        (sender, system_account(sender_lamports)),
        (recipient, system_account(exempt_minimum())),
    ];

    let mut mollusk = Mollusk::default();
    mollusk.config.panic = false;

    let result =
        mollusk.process_and_validate_transaction_instructions(&[instruction], &accounts, &[], None);

    assert_eq!(
        result.program_result,
        TransactionProgramResult::TransactionError(TransactionError::InsufficientFundsForRent {
            account_index: 0
        })
    );
    assert_eq!(result.raw_result, Ok(()));
}

#[test]
fn test_rent_checks_can_be_disabled() {
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let sender_lamports = exempt_minimum() + 1_000;
    let transfer_amount = 2_000;

    let instruction = transfer(&sender, &recipient, transfer_amount);
    let accounts = [
        (sender, system_account(sender_lamports)),
        (recipient, system_account(exempt_minimum())),
    ];

    let mut mollusk = Mollusk::default();
    mollusk.config.rent_exempt_checks = false;

    mollusk.process_and_validate_instruction(
        &instruction,
        &accounts,
        &[
            Check::success(),
            Check::account(&sender)
                .lamports(sender_lamports - transfer_amount)
                .build(),
        ],
    );
}

#[test]
#[should_panic(expected = "left in a rent state the runtime rejects")]
fn test_funding_new_account_below_minimum_fails() {
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let instruction = transfer(&sender, &recipient, 1);
    let accounts = [
        (sender, system_account(exempt_minimum() * 2)),
        (recipient, Account::default()),
    ];

    Mollusk::default().process_and_validate_instruction(&instruction, &accounts, &[]);
}

#[test]
#[should_panic(expected = "left in a rent state the runtime rejects")]
fn test_rent_checks_apply_per_chain_element() {
    let alice = Pubkey::new_unique();
    let bob = Pubkey::new_unique();

    let alice_lamports = exempt_minimum() - 500;

    let instruction_one = transfer(&bob, &alice, 200);
    let instruction_two = transfer(&alice, &bob, 100);

    let accounts = [
        (alice, system_account(alice_lamports)),
        (bob, system_account(exempt_minimum() * 2)),
    ];

    let checks: &[Check] = &[];

    Mollusk::default().process_and_validate_instruction_chain(
        &[(&instruction_one, checks), (&instruction_two, checks)],
        &accounts,
    );
}

#[test]
fn test_draining_rent_exempt_account_with_data_passes() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "test_program_primary");

    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();

    let data_len = 100;
    let source_lamports = mollusk.sysvars.rent.minimum_balance(data_len);
    let destination_lamports = exempt_minimum();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[6],
        vec![
            AccountMeta::new(source, false),
            AccountMeta::new(destination, false),
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (source, Account::new(source_lamports, data_len, &program_id)),
            (destination, system_account(destination_lamports)),
        ],
        &[
            Check::success(),
            Check::account(&source).lamports(0).space(data_len).build(),
            Check::account(&destination)
                .lamports(destination_lamports + source_lamports)
                .build(),
        ],
    );
}
