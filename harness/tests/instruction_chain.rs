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
fn test_transfers() {
    let mollusk = Mollusk::default();

    let alice = Pubkey::new_unique();
    let bob = Pubkey::new_unique();
    let carol = Pubkey::new_unique();
    let dave = Pubkey::new_unique();

    let starting_lamports = 500_000_000;

    let alice_to_bob = 100_000_000;
    let bob_to_carol = 50_000_000;
    let bob_to_dave = 50_000_000;

    mollusk.process_and_validate_instruction_chain(
        &[
            (
                // 0: Alice to Bob
                &solana_system_interface::instruction::transfer(&alice, &bob, alice_to_bob),
                &[
                    Check::success(),
                    Check::account(&alice)
                        .lamports(starting_lamports - alice_to_bob) // Alice pays
                        .build(),
                    Check::account(&bob)
                        .lamports(starting_lamports + alice_to_bob) // Bob receives
                        .build(),
                    Check::account(&carol)
                        .lamports(starting_lamports) // Unchanged
                        .build(),
                    Check::account(&dave)
                        .lamports(starting_lamports) // Unchanged
                        .build(),
                ],
            ),
            (
                // 1: Bob to Carol
                &solana_system_interface::instruction::transfer(&bob, &carol, bob_to_carol),
                &[
                    Check::success(),
                    Check::account(&alice)
                        .lamports(starting_lamports - alice_to_bob) // Unchanged
                        .build(),
                    Check::account(&bob)
                        .lamports(starting_lamports + alice_to_bob - bob_to_carol) // Bob pays
                        .build(),
                    Check::account(&carol)
                        .lamports(starting_lamports + bob_to_carol) // Carol receives
                        .build(),
                    Check::account(&dave)
                        .lamports(starting_lamports) // Unchanged
                        .build(),
                ],
            ),
            (
                // 2: Bob to Dave
                &solana_system_interface::instruction::transfer(&bob, &dave, bob_to_dave),
                &[
                    Check::success(),
                    Check::account(&alice)
                        .lamports(starting_lamports - alice_to_bob) // Unchanged
                        .build(),
                    Check::account(&bob)
                        .lamports(starting_lamports + alice_to_bob - bob_to_carol - bob_to_dave) // Bob pays
                        .build(),
                    Check::account(&carol)
                        .lamports(starting_lamports + bob_to_carol) // Unchanged
                        .build(),
                    Check::account(&dave)
                        .lamports(starting_lamports + bob_to_dave) // Dave receives
                        .build(),
                ],
            ),
        ],
        &[
            (alice, system_account_with_lamports(starting_lamports)),
            (bob, system_account_with_lamports(starting_lamports)),
            (carol, system_account_with_lamports(starting_lamports)),
            (dave, system_account_with_lamports(starting_lamports)),
        ],
    );
}

#[test]
fn test_mixed() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();

    let mollusk = Mollusk::new(&program_id, "test_program_primary");

    // First, for two target accounts:
    // 1. Credit with rent-exempt lamports (for 8 bytes of data).
    // 2. Allocate space.
    // 3. Assign to the program.
    // 4. Write some data.
    //
    // Then, close the first account.
    let payer = Pubkey::new_unique();
    let target1 = Pubkey::new_unique();
    let target2 = Pubkey::new_unique();

    let data = &[12; 8];
    let space = data.len();
    let lamports = mollusk.sysvars.rent.minimum_balance(space);

    let ix_transfer_to_1 =
        solana_system_interface::instruction::transfer(&payer, &target1, lamports);
    let ix_transfer_to_2 =
        solana_system_interface::instruction::transfer(&payer, &target2, lamports);
    let ix_allocate_1 = solana_system_interface::instruction::allocate(&target1, space as u64);
    let ix_allocate_2 = solana_system_interface::instruction::allocate(&target2, space as u64);
    let ix_assign_1 = solana_system_interface::instruction::assign(&target1, &program_id);
    let ix_assign_2 = solana_system_interface::instruction::assign(&target2, &program_id);
    let ix_write_data_to_1 = {
        let mut instruction_data = vec![1];
        instruction_data.extend_from_slice(data);
        Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![AccountMeta::new(target1, true)],
        )
    };
    let ix_write_data_to_2 = {
        let mut instruction_data = vec![1];
        instruction_data.extend_from_slice(data);
        Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![AccountMeta::new(target2, true)],
        )
    };
    let ix_close_1 = Instruction::new_with_bytes(
        program_id,
        &[3],
        vec![
            AccountMeta::new(target1, true),
            AccountMeta::new(solana_sdk_ids::incinerator::id(), false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::id(), false),
        ],
    );

    mollusk.process_and_validate_instruction_chain(
        &[
            (&ix_transfer_to_1, &[]),
            (&ix_transfer_to_2, &[]),
            (&ix_allocate_1, &[]),
            (&ix_allocate_2, &[]),
            (&ix_assign_1, &[]),
            (&ix_assign_2, &[]),
            (&ix_write_data_to_1, &[]),
            (&ix_write_data_to_2, &[]),
            (
                &ix_close_1,
                // Just check the final result.
                &[
                    Check::success(),
                    Check::account(&target1).closed().build(),
                    Check::account(&target2)
                        .data(data)
                        .lamports(lamports)
                        .owner(&program_id)
                        .build(),
                ],
            ),
        ],
        &[
            (payer, system_account_with_lamports(lamports * 4)),
            (target1, Account::default()),
            (target2, Account::default()),
            (solana_sdk_ids::incinerator::id(), Account::default()),
            keyed_account_for_system_program(),
        ],
    );
}
