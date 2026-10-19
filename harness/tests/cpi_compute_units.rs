#![cfg(feature = "cpi-compute-units")]

use {
    mollusk_svm::{
        program::{create_program_account_loader_v3, loader_keys::LOADER_V3},
        result::Check,
        Mollusk,
    },
    solana_account::Account,
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
};

struct Setup {
    mollusk: Mollusk,
    program_id: Pubkey,
    cpi_target_program_id: Pubkey,
    key: Pubkey,
    accounts: Vec<(Pubkey, Account)>,
}

fn setup() -> Setup {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let cpi_target_program_id = Pubkey::new_unique();

    let mut mollusk = Mollusk::new(&program_id, "test_program_primary");
    mollusk.add_program_with_loader(
        &cpi_target_program_id,
        "test_program_cpi_target",
        &LOADER_V3,
    );

    let space = 5;
    let lamports = mollusk.sysvars.rent.minimum_balance(space);
    let key = Pubkey::new_unique();

    let accounts = vec![
        (key, Account::new(lamports, space, &cpi_target_program_id)),
        (
            cpi_target_program_id,
            create_program_account_loader_v3(&cpi_target_program_id),
        ),
    ];

    Setup {
        mollusk,
        program_id,
        cpi_target_program_id,
        key,
        accounts,
    }
}

fn invoke_data(discriminator: u8, target: &Pubkey, data: &[u8]) -> Vec<u8> {
    let mut instruction_data = vec![discriminator];
    instruction_data.extend_from_slice(target.as_ref());
    instruction_data.extend_from_slice(data);
    instruction_data
}

#[test]
fn test_cpi_compute_units() {
    let Setup {
        mollusk,
        program_id,
        cpi_target_program_id,
        key,
        accounts,
    } = setup();

    let data = [1, 2, 3, 4, 5];

    let instruction = Instruction::new_with_bytes(
        program_id,
        &invoke_data(4, &cpi_target_program_id, &data),
        vec![
            AccountMeta::new(key, true),
            AccountMeta::new_readonly(cpi_target_program_id, false),
        ],
    );
    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    assert_eq!(result.cpi_compute_units.len(), 1);
    let cpi = &result.cpi_compute_units[0];
    assert_eq!(cpi.stack_height, 2);
    assert_eq!(cpi.program_id, cpi_target_program_id);
    assert!(cpi.compute_units_consumed < result.compute_units_consumed);

    let direct_instruction = Instruction::new_with_bytes(
        cpi_target_program_id,
        &data,
        vec![AccountMeta::new(key, true)],
    );
    let direct_result = mollusk.process_and_validate_instruction(
        &direct_instruction,
        &accounts,
        &[Check::success()],
    );
    assert!(direct_result.cpi_compute_units.is_empty());
    assert!(
        cpi.compute_units_consumed
            >= direct_result.compute_units_consumed + mollusk.compute_budget.invoke_units
    );
}

#[test]
fn test_nested_cpi_compute_units() {
    let Setup {
        mollusk,
        program_id,
        cpi_target_program_id,
        key,
        mut accounts,
    } = setup();
    accounts.push((program_id, create_program_account_loader_v3(&program_id)));

    let instruction = Instruction::new_with_bytes(
        program_id,
        &invoke_data(
            7,
            &program_id,
            &invoke_data(7, &cpi_target_program_id, &[1, 2, 3, 4, 5]),
        ),
        vec![
            AccountMeta::new(key, true),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new_readonly(cpi_target_program_id, false),
        ],
    );
    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);

    assert_eq!(result.cpi_compute_units.len(), 2);
    let (outer, inner) = (&result.cpi_compute_units[0], &result.cpi_compute_units[1]);
    assert_eq!(outer.stack_height, 2);
    assert_eq!(outer.program_id, program_id);
    assert_eq!(inner.stack_height, 3);
    assert_eq!(inner.program_id, cpi_target_program_id);
    assert!(inner.compute_units_consumed < outer.compute_units_consumed);
    assert!(outer.compute_units_consumed < result.compute_units_consumed);
}

#[test]
fn test_instruction_chain_cpi_compute_units() {
    let Setup {
        mollusk,
        program_id,
        cpi_target_program_id,
        key,
        accounts,
    } = setup();

    let cpi_instruction = Instruction::new_with_bytes(
        program_id,
        &invoke_data(4, &cpi_target_program_id, &[1, 2, 3, 4, 5]),
        vec![
            AccountMeta::new(key, true),
            AccountMeta::new_readonly(cpi_target_program_id, false),
        ],
    );
    let noop_instruction = Instruction::new_with_bytes(program_id, &[0], vec![]);

    let single = mollusk.process_instruction(&cpi_instruction, &accounts);
    let result = mollusk.process_and_validate_instruction_chain(
        &[
            (&cpi_instruction, &[Check::success()]),
            (&noop_instruction, &[Check::success()]),
            (&cpi_instruction, &[Check::success()]),
        ],
        &accounts,
    );

    assert_eq!(
        result.cpi_compute_units,
        [
            single.cpi_compute_units[0].clone(),
            single.cpi_compute_units[0].clone()
        ]
    );
}

#[test]
fn test_no_cpi_compute_units() {
    let Setup {
        mollusk,
        program_id,
        ..
    } = setup();

    let instruction = Instruction::new_with_bytes(program_id, &[0], vec![]);
    let result = mollusk.process_and_validate_instruction(&instruction, &[], &[Check::success()]);

    assert!(result.cpi_compute_units.is_empty());
}
