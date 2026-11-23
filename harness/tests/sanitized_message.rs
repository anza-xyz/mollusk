use {
    mollusk_svm::{program::keyed_account_for_system_program, Mollusk},
    solana_account::Account,
    solana_hash::Hash,
    solana_instruction_error::InstructionError,
    solana_message::{
        compiled_instruction::CompiledInstruction,
        v0::{self, LoadedAddresses, LoadedMessage},
        LegacyMessage, Message, MessageHeader, SanitizedMessage,
    },
    solana_pubkey::Pubkey,
    solana_transaction_error::TransactionError,
    std::collections::HashSet,
};

const BASE_LAMPORTS: u64 = 100_000_000;
const TRANSFER_AMOUNT: u64 = 42_000;

fn system_account(lamports: u64) -> Account {
    Account::new(lamports, 0, &solana_sdk_ids::system_program::id())
}

fn transfer_data(lamports: u64) -> Vec<u8> {
    bincode::serialize(
        &solana_system_interface::instruction::SystemInstruction::Transfer { lamports },
    )
    .unwrap()
}

/// A v0 message transferring from the static fee payer to a recipient loaded
/// from an address lookup table.
fn v0_transfer_to_loaded_recipient(
    payer: &Pubkey,
    recipient: &Pubkey,
    is_recipient_writable: bool,
) -> SanitizedMessage {
    let (system_program, _) = keyed_account_for_system_program();
    let (writable_indexes, readonly_indexes, loaded_addresses) = if is_recipient_writable {
        (
            vec![0],
            vec![],
            LoadedAddresses {
                writable: vec![*recipient],
                readonly: vec![],
            },
        )
    } else {
        (
            vec![],
            vec![0],
            LoadedAddresses {
                writable: vec![],
                readonly: vec![*recipient],
            },
        )
    };
    let message = v0::Message {
        header: MessageHeader {
            num_required_signatures: 1,
            num_readonly_signed_accounts: 0,
            num_readonly_unsigned_accounts: 1,
        },
        account_keys: vec![*payer, system_program],
        recent_blockhash: Hash::default(),
        instructions: vec![CompiledInstruction::new_from_raw_parts(
            /* program_id_index */ 1,
            transfer_data(TRANSFER_AMOUNT),
            /* accounts */ vec![0, 2],
        )],
        // The lookup table itself is never read, since the loaded addresses
        // are provided directly.
        address_table_lookups: vec![v0::MessageAddressTableLookup {
            account_key: Pubkey::new_unique(),
            writable_indexes,
            readonly_indexes,
        }],
    };
    SanitizedMessage::V0(LoadedMessage::new(
        message,
        loaded_addresses,
        &HashSet::new(),
    ))
}

#[test]
fn test_legacy_message_matches_transaction_instructions() {
    let mollusk = Mollusk::default();

    let payer = Pubkey::new_unique();
    let recipient_a = Pubkey::new_unique();
    let recipient_b = Pubkey::new_unique();

    let instructions = [
        solana_system_interface::instruction::transfer(&payer, &recipient_a, TRANSFER_AMOUNT),
        solana_system_interface::instruction::transfer(&payer, &recipient_b, TRANSFER_AMOUNT),
    ];
    let accounts = [
        (payer, system_account(BASE_LAMPORTS)),
        (recipient_a, system_account(BASE_LAMPORTS)),
        (recipient_b, system_account(BASE_LAMPORTS)),
        keyed_account_for_system_program(),
    ];

    let expected = mollusk.process_transaction_instructions(&instructions, &accounts, Some(&payer));

    let message = SanitizedMessage::Legacy(LegacyMessage::new(
        Message::new(&instructions, Some(&payer)),
        &HashSet::new(),
    ));
    let result = mollusk.process_sanitized_message(&message, &accounts);

    assert!(result.program_result.is_ok());
    assert_eq!(result.program_result, expected.program_result);
    assert_eq!(result.raw_result, expected.raw_result);
    assert_eq!(
        result.compute_units_consumed,
        expected.compute_units_consumed
    );
    assert_eq!(result.resulting_accounts, expected.resulting_accounts);
    assert_eq!(
        result.get_account(&payer).unwrap().lamports,
        BASE_LAMPORTS - 2 * TRANSFER_AMOUNT,
    );
}

#[test]
fn test_v0_message_writable_loaded_address() {
    let mollusk = Mollusk::default();

    let payer = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let message =
        v0_transfer_to_loaded_recipient(&payer, &recipient, /* is_recipient_writable */ true);
    let accounts = [
        (payer, system_account(BASE_LAMPORTS)),
        (recipient, system_account(BASE_LAMPORTS)),
        keyed_account_for_system_program(),
    ];

    let result = mollusk.process_sanitized_message(&message, &accounts);

    assert!(result.program_result.is_ok());
    assert_eq!(
        result.get_account(&payer).unwrap().lamports,
        BASE_LAMPORTS - TRANSFER_AMOUNT,
    );
    assert_eq!(
        result.get_account(&recipient).unwrap().lamports,
        BASE_LAMPORTS + TRANSFER_AMOUNT,
    );
}

#[test]
fn test_v0_message_readonly_loaded_address() {
    let mollusk = Mollusk::default();

    let payer = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    // The recipient is loaded as read-only, so the transfer must fail even
    // though nothing in the instruction itself marks it read-only.
    let message =
        v0_transfer_to_loaded_recipient(&payer, &recipient, /* is_recipient_writable */ false);
    let accounts = [
        (payer, system_account(BASE_LAMPORTS)),
        (recipient, system_account(BASE_LAMPORTS)),
        keyed_account_for_system_program(),
    ];

    let result = mollusk.process_sanitized_message(&message, &accounts);

    assert_eq!(
        result.raw_result,
        Err(TransactionError::InstructionError(
            0,
            InstructionError::ReadonlyLamportChange
        )),
    );
    assert_eq!(result.resulting_accounts, accounts);
}

#[test]
fn test_reserved_account_key_is_demoted() {
    let mollusk = Mollusk::default();

    let payer = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let instruction =
        solana_system_interface::instruction::transfer(&payer, &recipient, TRANSFER_AMOUNT);
    let accounts = [
        (payer, system_account(BASE_LAMPORTS)),
        (recipient, system_account(BASE_LAMPORTS)),
        keyed_account_for_system_program(),
    ];

    // Reserving the recipient demotes it to read-only, as the runtime does.
    let message = SanitizedMessage::Legacy(LegacyMessage::new(
        Message::new(&[instruction], Some(&payer)),
        &HashSet::from([recipient]),
    ));

    let result = mollusk.process_sanitized_message(&message, &accounts);

    assert_eq!(
        result.raw_result,
        Err(TransactionError::InstructionError(
            0,
            InstructionError::ReadonlyLamportChange
        )),
    );
}
