use {
    mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk},
    solana_account::Account,
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
};

fn system_account_with_lamports(lamports: u64) -> Account {
    Account::new(lamports, 0, &solana_sdk_ids::system_program::id())
}

#[test]
fn test_transfers_with_persisted_state() {
    let mollusk = Mollusk::default();

    let sender = Pubkey::new_unique();
    let intermediary = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let initial_balance = 100_000_000u64;
    let transfer_amount = 50_000_000u64;

    mollusk.process_and_validate_transaction_instructions(
        &[
            (
                &solana_system_interface::instruction::transfer(
                    &sender,
                    &intermediary,
                    transfer_amount,
                ),
                &[
                    Check::success(),
                    Check::account(&sender)
                        .lamports(initial_balance - transfer_amount)
                        .build(),
                    Check::account(&intermediary)
                        .lamports(transfer_amount)
                        .build(),
                ],
            ),
            (
                &solana_system_interface::instruction::transfer(
                    &intermediary,
                    &recipient,
                    transfer_amount,
                ),
                &[
                    Check::success(),
                    Check::account(&intermediary).lamports(0).build(),
                    Check::account(&recipient).lamports(transfer_amount).build(),
                ],
            ),
        ],
        &[
            (sender, system_account_with_lamports(initial_balance)),
            (intermediary, system_account_with_lamports(0)),
            (recipient, system_account_with_lamports(0)),
        ],
    );
}

#[test]
fn test_multi_program_transaction() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "test_program_primary");

    let payer = Pubkey::new_unique();
    let target = Pubkey::new_unique();
    let data = &[42; 8];
    let space = data.len();
    let lamports = mollusk.sysvars.rent.minimum_balance(space);

    let ix_transfer = solana_system_interface::instruction::transfer(&payer, &target, lamports);
    let ix_allocate = solana_system_interface::instruction::allocate(&target, space as u64);
    let ix_assign = solana_system_interface::instruction::assign(&target, &program_id);

    let ix_write_data = {
        let mut instruction_data = vec![1];
        instruction_data.extend_from_slice(data);
        Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![AccountMeta::new(target, true)],
        )
    };

    mollusk.process_and_validate_transaction_instructions(
        &[
            (&ix_transfer, &[Check::success()]),
            (&ix_allocate, &[Check::success()]),
            (&ix_assign, &[Check::success()]),
            (
                &ix_write_data,
                &[
                    Check::success(),
                    Check::account(&target)
                        .data(data)
                        .lamports(lamports)
                        .owner(&program_id)
                        .build(),
                ],
            ),
        ],
        &[
            (payer, system_account_with_lamports(lamports * 2)),
            (target, Account::default()),
            keyed_account_for_system_program(),
        ],
    );
}

#[test]
fn test_compute_units_tracked() {
    let mut mollusk = Mollusk::default();
    mollusk.compute_budget.compute_unit_limit = 1000;

    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let result = mollusk.process_transaction_instructions(
        &[solana_system_interface::instruction::transfer(
            &sender, &recipient, 100,
        )],
        &[
            (sender, system_account_with_lamports(1000)),
            (recipient, system_account_with_lamports(0)),
        ],
    );

    assert!(result.compute_units_consumed > 0);
    assert!(result.compute_units_consumed <= 1000);
}

#[test]
fn test_compute_units_accumulate_across_instructions() {
    let mollusk = Mollusk::default();

    let alice = Pubkey::new_unique();
    let bob = Pubkey::new_unique();
    let carol = Pubkey::new_unique();

    let result = mollusk.process_transaction_instructions(
        &[
            solana_system_interface::instruction::transfer(&alice, &bob, 1000),
            solana_system_interface::instruction::transfer(&bob, &carol, 500),
        ],
        &[
            (alice, system_account_with_lamports(10_000)),
            (bob, system_account_with_lamports(0)),
            (carol, system_account_with_lamports(0)),
        ],
    );

    assert!(
        result.compute_units_consumed >= 150,
        "Expected CU >= 150 for two transfers, got {}",
        result.compute_units_consumed
    );
}

#[test]
fn test_failure_stops_instruction_chain() {
    let mollusk = Mollusk::default();

    let alice = Pubkey::new_unique();
    let bob = Pubkey::new_unique();
    let carol = Pubkey::new_unique();

    let initial_balance = 1_000_000u64;

    // First transfer succeeds, second fails (insufficient funds), third never runs
    let result = mollusk.process_transaction_instructions(
        &[
            solana_system_interface::instruction::transfer(&alice, &bob, 100),
            solana_system_interface::instruction::transfer(&bob, &carol, 999_999),
            solana_system_interface::instruction::transfer(&alice, &carol, 50),
        ],
        &[
            (alice, system_account_with_lamports(initial_balance)),
            (bob, system_account_with_lamports(0)),
            (carol, system_account_with_lamports(0)),
        ],
    );

    assert!(result.program_result.is_err());

    let alice_account = result
        .resulting_accounts
        .iter()
        .find(|(pk, _)| pk == &alice)
        .map(|(_, acc)| acc.lamports);
    assert_eq!(alice_account, Some(initial_balance));
}

#[test]
fn test_missing_signer_fails() {
    let mollusk = Mollusk::default();

    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let mut ix = solana_system_interface::instruction::transfer(&sender, &recipient, 100);
    ix.accounts[0].is_signer = false;

    let result = mollusk.process_transaction_instructions(
        &[ix],
        &[
            (sender, system_account_with_lamports(1_000_000)),
            (recipient, system_account_with_lamports(0)),
        ],
    );

    assert!(result.program_result.is_err());
}

#[test]
fn test_many_instructions_in_transaction() {
    let mollusk = Mollusk::default();

    let sender = Pubkey::new_unique();
    let recipients: Vec<Pubkey> = (0..10).map(|_| Pubkey::new_unique()).collect();

    let initial_balance = 10_000_000u64;
    let transfer_amount = 1000u64;

    let instructions: Vec<Instruction> = recipients
        .iter()
        .map(|recipient| {
            solana_system_interface::instruction::transfer(&sender, recipient, transfer_amount)
        })
        .collect();

    let mut accounts = vec![(sender, system_account_with_lamports(initial_balance))];
    for recipient in &recipients {
        accounts.push((*recipient, system_account_with_lamports(0)));
    }

    let result = mollusk.process_transaction_instructions(&instructions, &accounts);

    assert!(result.program_result.is_ok());

    let sender_account = result
        .resulting_accounts
        .iter()
        .find(|(pk, _)| pk == &sender)
        .map(|(_, acc)| acc.lamports);
    assert_eq!(
        sender_account,
        Some(initial_balance - (transfer_amount * 10))
    );
}
