use {
    agave_feature_set::FeatureSet,
    mollusk_svm::{
        file::load_program_elf,
        program::{create_program_account_pair_loader_v3, keyed_account_for_system_program},
        Mollusk,
    },
    mollusk_svm_replay::{ReplayContext, ReplayError, Replayer},
    solana_account::Account,
    solana_instruction::{AccountMeta, Instruction},
    solana_instruction_error::InstructionError,
    solana_loader_v3_interface::get_program_data_address,
    solana_message::{v0::LoadedAddresses, Message, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_transaction_error::TransactionError,
    std::collections::HashSet,
};

const BASE_LAMPORTS: u64 = 1_000_000_000;
const FEE: u64 = 5_000;
const DATA: &[u8] = &[7; 8];

fn primary_elf() -> Vec<u8> {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");
    load_program_elf("test_program_primary")
}

fn instructions_sysvar_elf() -> Vec<u8> {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");
    load_program_elf("test_program_instructions_sysvar")
}

fn system_account(lamports: u64) -> Account {
    Account::new(lamports, 0, &solana_sdk_ids::system_program::id())
}

/// Accounts for a loader-v3 program, plus the sysvars a real context carries.
fn program_accounts(program_id: &Pubkey, elf: &[u8]) -> Vec<(Pubkey, Account)> {
    let (program_account, programdata_account) =
        create_program_account_pair_loader_v3(program_id, elf);
    let mut accounts = vec![
        (*program_id, program_account),
        (get_program_data_address(program_id), programdata_account),
    ];
    accounts.extend(Mollusk::default().sysvars.get_all_keyed_sysvar_accounts());
    accounts
}

fn context(
    instructions: &[Instruction],
    payer: &Pubkey,
    accounts: Vec<(Pubkey, Account)>,
    recorded_status: Result<(), TransactionError>,
) -> ReplayContext {
    ReplayContext {
        slot: 1,
        message: VersionedMessage::Legacy(Message::new(instructions, Some(payer))),
        loaded_addresses: LoadedAddresses::default(),
        reserved_account_keys: HashSet::new(),
        accounts,
        feature_set: FeatureSet::all_enabled(),
        epoch_stake: Default::default(),
        fee: FEE,
        recorded_status,
    }
}

/// A transaction in which the primary program writes `DATA` into a signer
/// account, after any `leading_instructions`.
fn write_data_context_with(
    program_id: &Pubkey,
    elf: &[u8],
    leading_instructions: Vec<Instruction>,
    recorded_status: Result<(), TransactionError>,
) -> ReplayContext {
    let payer = Pubkey::new_unique();
    let target = Pubkey::new_unique();

    let mut instruction_data = vec![1];
    instruction_data.extend_from_slice(DATA);
    let mut instructions = leading_instructions;
    instructions.push(Instruction::new_with_bytes(
        *program_id,
        &instruction_data,
        vec![AccountMeta::new(target, true)],
    ));

    let mut accounts = vec![
        (payer, system_account(BASE_LAMPORTS)),
        (target, Account::new(BASE_LAMPORTS, DATA.len(), program_id)),
    ];
    accounts.extend(program_accounts(program_id, elf));

    context(&instructions, &payer, accounts, recorded_status)
}

fn write_data_context(program_id: &Pubkey, elf: &[u8]) -> ReplayContext {
    write_data_context_with(program_id, elf, vec![], Ok(()))
}

#[test]
fn test_candidate_matches_recorded_result() {
    let program_id = Pubkey::new_unique();
    let elf = primary_elf();
    let replayer = Replayer::new(write_data_context(&program_id, &elf)).unwrap();

    assert_eq!(replayer.program_ids().collect::<Vec<_>>(), [&program_id]);

    let report = replayer.verify(&program_id, &elf).unwrap();

    assert!(report.candidate_matches());
    assert_eq!(report.recorded, Ok(()));
    assert_eq!(report.baseline.raw_result, Ok(()));
    assert_eq!(
        report.candidate.resulting_accounts,
        report.baseline.resulting_accounts
    );
}

#[test]
fn test_candidate_with_different_behavior_mismatches() {
    let program_id = Pubkey::new_unique();
    let replayer = Replayer::new(write_data_context(&program_id, &primary_elf())).unwrap();

    // The instructions sysvar test program expects the sysvar as its last
    // account, so it rejects the transaction.
    let report = replayer
        .verify(&program_id, &instructions_sysvar_elf())
        .unwrap();

    assert!(!report.candidate_matches());
    assert_eq!(report.baseline.raw_result, Ok(()));
    assert_eq!(
        report.candidate.raw_result,
        Err(TransactionError::InstructionError(
            0,
            InstructionError::InvalidAccountOwner
        )),
    );
}

#[test]
fn test_sanity_check_failure() {
    let program_id = Pubkey::new_unique();
    let elf = primary_elf();

    // The ledger claims the transaction failed, which the historical program
    // does not reproduce.
    let recorded = Err(TransactionError::InstructionError(
        0,
        InstructionError::InvalidAccountData,
    ));
    let replayer = Replayer::new(ReplayContext {
        recorded_status: recorded.clone(),
        ..write_data_context(&program_id, &elf)
    })
    .unwrap();

    let err = replayer.verify(&program_id, &elf).unwrap_err();

    assert!(matches!(
        err,
        ReplayError::SanityCheckFailed { recorded: r, replayed: Ok(()) } if r == recorded
    ));
}

#[test]
fn test_fee_is_deducted_before_execution() {
    let payer = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    // The payer can afford the transfer, but not the transfer and the fee.
    let transfer_amount = BASE_LAMPORTS - FEE + 1;
    let instruction =
        solana_system_interface::instruction::transfer(&payer, &recipient, transfer_amount);
    let accounts = vec![
        (payer, system_account(BASE_LAMPORTS)),
        (recipient, system_account(BASE_LAMPORTS)),
        keyed_account_for_system_program(),
    ];
    // `SystemError::ResultWithNegativeLamports`.
    let recorded = Err(TransactionError::InstructionError(
        0,
        InstructionError::Custom(1),
    ));

    let replayer =
        Replayer::new(context(&[instruction], &payer, accounts, recorded.clone())).unwrap();
    let result = replayer.replay();

    assert_eq!(result.raw_result, recorded);
    assert_eq!(
        result.get_account(&payer).unwrap().lamports,
        BASE_LAMPORTS - FEE
    );
}

#[test]
fn test_compute_unit_limit_is_applied() {
    // `SetComputeUnitLimit(1)`, which the compute budget program itself cannot
    // fit in.
    let set_compute_unit_limit = Instruction::new_with_bytes(
        solana_sdk_ids::compute_budget::id(),
        &[2, 1, 0, 0, 0],
        vec![],
    );
    let exceeded = Err(TransactionError::InstructionError(
        0,
        InstructionError::ComputationalBudgetExceeded,
    ));

    let program_id = Pubkey::new_unique();
    let replayer = Replayer::new(write_data_context_with(
        &program_id,
        &primary_elf(),
        vec![set_compute_unit_limit],
        exceeded.clone(),
    ))
    .unwrap();

    assert_eq!(replayer.replay().raw_result, exceeded);
}

#[test]
fn test_program_not_in_transaction() {
    let program_id = Pubkey::new_unique();
    let elf = primary_elf();
    let replayer = Replayer::new(write_data_context(&program_id, &elf)).unwrap();

    let other_program_id = Pubkey::new_unique();
    let err = replayer
        .replay_with_program(&other_program_id, &elf)
        .unwrap_err();

    assert!(matches!(err, ReplayError::ProgramNotInTransaction(id) if id == other_program_id));
}

#[test]
fn test_durable_nonce_unsupported() {
    let payer = Pubkey::new_unique();
    let nonce = Pubkey::new_unique();

    let instruction = solana_system_interface::instruction::advance_nonce_account(&nonce, &payer);
    let accounts = vec![
        (payer, system_account(BASE_LAMPORTS)),
        (nonce, system_account(BASE_LAMPORTS)),
        keyed_account_for_system_program(),
    ];

    let result = Replayer::new(context(&[instruction], &payer, accounts, Ok(())));

    assert!(matches!(result, Err(ReplayError::UnsupportedDurableNonce)));
}

#[test]
fn test_transaction_not_executed() {
    let program_id = Pubkey::new_unique();

    let result = Replayer::new(ReplayContext {
        recorded_status: Err(TransactionError::InsufficientFundsForFee),
        ..write_data_context(&program_id, &primary_elf())
    });

    assert!(matches!(
        result,
        Err(ReplayError::TransactionNotExecuted(
            TransactionError::InsufficientFundsForFee
        ))
    ));
}

#[test]
fn test_unknown_candidate_feature() {
    let program_id = Pubkey::new_unique();
    let mut replayer = Replayer::new(write_data_context(&program_id, &primary_elf())).unwrap();

    let unknown = Pubkey::new_unique();
    let err = replayer
        .activate_candidate_features(&[unknown])
        .unwrap_err();

    assert!(matches!(err, ReplayError::UnknownFeature(id) if id == unknown));
}

#[test]
fn test_candidate_features_apply_only_to_candidate() {
    let feature_id = agave_feature_set::create_account_allow_prefund::id();
    let new_owner = Pubkey::new_unique();
    let prefunded = Pubkey::new_unique();

    // `CreateAccountAllowPrefund` is rejected while its feature is inactive.
    let create_prefunded = Instruction::new_with_bytes(
        solana_sdk_ids::system_program::id(),
        &bincode::serialize(
            &solana_system_interface::instruction::SystemInstruction::CreateAccountAllowPrefund {
                lamports: 0,
                space: 2,
                owner: new_owner,
            },
        )
        .unwrap(),
        vec![AccountMeta::new(prefunded, true)],
    );
    let rejected = Err(TransactionError::InstructionError(
        0,
        InstructionError::InvalidInstructionData,
    ));

    let program_id = Pubkey::new_unique();
    let elf = primary_elf();
    let mut context =
        write_data_context_with(&program_id, &elf, vec![create_prefunded], rejected.clone());
    context.feature_set.deactivate(&feature_id);
    context.accounts.extend([
        (
            prefunded,
            system_account(Mollusk::default().sysvars.rent.minimum_balance(2)),
        ),
        keyed_account_for_system_program(),
    ]);

    let mut replayer = Replayer::new(context).unwrap();
    assert!(replayer
        .verify(&program_id, &elf)
        .unwrap()
        .candidate_matches());

    replayer.activate_candidate_features(&[feature_id]).unwrap();
    let report = replayer.verify(&program_id, &elf).unwrap();

    // The sanity check still reproduces the historical failure, while the
    // candidate runs with the feature active.
    assert_eq!(report.baseline.raw_result, rejected);
    assert_eq!(report.candidate.raw_result, Ok(()));
    assert!(!report.candidate_matches());
    assert_eq!(
        report.candidate.get_account(&prefunded).unwrap().owner,
        new_owner
    );
}
