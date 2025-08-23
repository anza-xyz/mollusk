use {
    mollusk_svm::{result::Check, Mollusk},
    solana_account::{Account, ReadableAccount},
    solana_program_error::ProgramError,
    solana_pubkey::Pubkey,
    solana_system_interface::error::SystemError,
    solana_system_program::system_processor::DEFAULT_COMPUTE_UNITS,
    std::collections::HashMap,
};

#[test]
fn test_transfer_with_context() {
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let base_lamports = 100_000_000u64;
    let transfer_amount = 42_000u64;

    // Create context with HashMap account store
    let mollusk = Mollusk::default();
    let mut account_store = HashMap::new();

    // Initialize accounts in the store
    account_store.insert(
        sender,
        Account::new(base_lamports, 0, &solana_sdk_ids::system_program::id()),
    );
    account_store.insert(
        recipient,
        Account::new(base_lamports, 0, &solana_sdk_ids::system_program::id()),
    );

    let context = mollusk.with_context(account_store);

    // Process the transfer instruction
    let result = context.process_and_validate_instruction(
        &solana_system_interface::instruction::transfer(&sender, &recipient, transfer_amount),
        &[
            Check::success(),
            Check::compute_units(DEFAULT_COMPUTE_UNITS),
        ],
    );

    // Verify the result was successful
    assert!(!result.program_result.is_err());

    // Verify account states were persisted correctly in the account store
    let store = context.account_store.borrow();

    let sender_account = store.get(&sender).unwrap();
    assert_eq!(sender_account.lamports(), base_lamports - transfer_amount);

    let recipient_account = store.get(&recipient).unwrap();
    assert_eq!(
        recipient_account.lamports(),
        base_lamports + transfer_amount
    );
}

#[test]
fn test_multiple_transfers_with_persistent_state() {
    let alice = Pubkey::new_unique();
    let bob = Pubkey::new_unique();
    let charlie = Pubkey::new_unique();

    let initial_lamports = 1_000_000u64;
    let transfer1_amount = 200_000u64;
    let transfer2_amount = 150_000u64;

    // Create context with HashMap account store
    let mollusk = Mollusk::default();
    let mut account_store = HashMap::new();

    // Initialize accounts
    account_store.insert(
        alice,
        Account::new(initial_lamports, 0, &solana_sdk_ids::system_program::id()),
    );
    account_store.insert(
        bob,
        Account::new(initial_lamports, 0, &solana_sdk_ids::system_program::id()),
    );
    account_store.insert(
        charlie,
        Account::new(initial_lamports, 0, &solana_sdk_ids::system_program::id()),
    );

    let context = mollusk.with_context(account_store);

    let checks = vec![
        Check::success(),
        Check::compute_units(DEFAULT_COMPUTE_UNITS),
    ];

    // First transfer: Alice -> Bob
    let instruction1 =
        solana_system_interface::instruction::transfer(&alice, &bob, transfer1_amount);
    let result1 = context.process_and_validate_instruction(&instruction1, &checks);
    assert!(!result1.program_result.is_err());

    // Second transfer: Bob -> Charlie
    let instruction2 =
        solana_system_interface::instruction::transfer(&bob, &charlie, transfer2_amount);
    let result2 = context.process_and_validate_instruction(&instruction2, &checks);
    assert!(!result2.program_result.is_err());

    // Verify final account states
    let store = context.account_store.borrow();

    let alice_account = store.get(&alice).unwrap();
    assert_eq!(
        alice_account.lamports(),
        initial_lamports - transfer1_amount
    );

    let bob_account = store.get(&bob).unwrap();
    assert_eq!(
        bob_account.lamports(),
        initial_lamports + transfer1_amount - transfer2_amount
    );

    let charlie_account = store.get(&charlie).unwrap();
    assert_eq!(
        charlie_account.lamports(),
        initial_lamports + transfer2_amount
    );
}

#[test]
fn test_account_store_sysvar_account() {
    let mollusk = Mollusk::default();
    let context = mollusk.with_context(HashMap::new());

    // Use Clock sysvar as an example.
    let clock_pubkey = solana_sdk_ids::sysvar::clock::id();
    let recipient = Pubkey::new_unique();

    // Create an instruction that references the Clock sysvar.
    let instruction = solana_instruction::Instruction::new_with_bytes(
        solana_sdk_ids::system_program::id(),
        &[],
        vec![
            solana_instruction::AccountMeta::new_readonly(clock_pubkey, false),
            solana_instruction::AccountMeta::new(recipient, false),
        ],
    );

    // Process the instruction - this should load the Clock sysvar account.
    context.process_instruction(&instruction);

    // Verify the Clock sysvar was loaded correctly.
    let store = context.account_store.borrow();
    let clock_account = store.get(&clock_pubkey).expect("Clock sysvar should exist");

    // Verify it has the expected owner.
    assert_eq!(clock_account.owner, solana_sdk_ids::sysvar::id());
    // Verify it has data (Clock sysvar should have serialized Clock data).
    assert!(!clock_account.data.is_empty());
}

#[test]
fn test_account_store_program_account() {
    // Use the System Program as an example.
    let program_id = solana_sdk_ids::system_program::id();
    let mollusk = Mollusk::default();

    let context = mollusk.with_context(HashMap::new());
    let recipient = Pubkey::new_unique();

    // Create an instruction that references the program account.
    let instruction = solana_instruction::Instruction::new_with_bytes(
        solana_sdk_ids::bpf_loader_upgradeable::id(),
        &[],
        vec![
            solana_instruction::AccountMeta::new_readonly(program_id, false),
            solana_instruction::AccountMeta::new(recipient, false),
        ],
    );

    // Process the instruction - this should load the program account.
    context.process_instruction(&instruction);

    // Verify the program account was loaded correctly
    let store = context.account_store.borrow();
    let program_account = store
        .get(&program_id)
        .expect("Program account should exist");

    // Verify it has the expected owner (native loader for builtins).
    assert_eq!(program_account.owner, solana_sdk_ids::native_loader::id());
    // Verify it's marked as executable.
    assert!(program_account.executable);
}

#[test]
fn test_account_store_default_account() {
    let mollusk = Mollusk::default();
    let context = mollusk.with_context(HashMap::new());

    let non_existent_key = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    // Try to transfer from a non-existent account (should get default account)
    let instruction =
        solana_system_interface::instruction::transfer(&non_existent_key, &recipient, 1000);

    // This should fail because the default account has 0 lamports
    context.process_and_validate_instruction(
        &instruction,
        &[Check::err(ProgramError::Custom(
            SystemError::ResultWithNegativeLamports as u32,
        ))],
    );
}
