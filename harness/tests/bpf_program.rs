use {
    mollusk_svm::{
        program::{create_program_account_loader_v3, keyed_account_for_system_program},
        result::Check,
        Mollusk,
    },
    solana_account::Account,
    solana_instruction::{error::InstructionError, AccountMeta, Instruction},
    solana_program_error::ProgramError,
    solana_pubkey::Pubkey,
    solana_system_interface::error::SystemError,
};

#[test]
fn test_write_data() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();

    let mollusk = Mollusk::new(&program_id, "test_program_primary");

    let data = &[1, 2, 3, 4, 5];
    let space = data.len();
    let lamports = mollusk.sysvars.rent.minimum_balance(space);

    let key = Pubkey::new_unique();
    let account = Account::new(lamports, space, &program_id);

    let instruction = {
        let mut instruction_data = vec![1];
        instruction_data.extend_from_slice(data);
        Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![AccountMeta::new(key, true)],
        )
    };

    // Fail account not signer.
    {
        let mut account_not_signer_ix = instruction.clone();
        account_not_signer_ix.accounts[0].is_signer = false;

        mollusk.process_and_validate_instruction(
            &account_not_signer_ix,
            &[(key, account.clone())],
            &[Check::err(ProgramError::MissingRequiredSignature)],
        );
    }

    // Fail data too large.
    {
        let mut data_too_large_ix = instruction.clone();
        data_too_large_ix.data = vec![1; space + 2];

        mollusk.process_and_validate_instruction(
            &data_too_large_ix,
            &[(key, account.clone())],
            &[Check::err(ProgramError::AccountDataTooSmall)],
        );
    }

    // Success.
    mollusk.process_and_validate_instruction(
        &instruction,
        &[(key, account.clone())],
        &[
            Check::success(),
            Check::compute_units(336),
            Check::account(&key)
                .data(data)
                .lamports(lamports)
                .owner(&program_id)
                .space(space)
                .build(),
        ],
    );
}

#[test]
fn test_transfer() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();

    let mollusk = Mollusk::new(&program_id, "test_program_primary");

    let payer = Pubkey::new_unique();
    let payer_lamports = 100_000_000;
    let payer_account = Account::new(payer_lamports, 0, &solana_sdk_ids::system_program::id());

    let recipient = Pubkey::new_unique();
    let recipient_lamports = 0;
    let recipient_account =
        Account::new(recipient_lamports, 0, &solana_sdk_ids::system_program::id());

    let transfer_amount = 2_000_000_u64;

    let instruction = {
        let mut instruction_data = vec![2];
        instruction_data.extend_from_slice(&transfer_amount.to_le_bytes());
        Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(payer, true),
                AccountMeta::new(recipient, false),
                AccountMeta::new_readonly(solana_sdk_ids::system_program::id(), false),
            ],
        )
    };

    // Fail payer not signer.
    {
        let mut payer_not_signer_ix = instruction.clone();
        payer_not_signer_ix.accounts[0].is_signer = false;

        mollusk.process_and_validate_instruction(
            &payer_not_signer_ix,
            &[
                (payer, payer_account.clone()),
                (recipient, recipient_account.clone()),
                keyed_account_for_system_program(),
            ],
            &[Check::err(ProgramError::MissingRequiredSignature)],
        );
    }

    // Fail insufficient lamports.
    {
        mollusk.process_and_validate_instruction(
            &instruction,
            &[
                (payer, Account::default()),
                (recipient, recipient_account.clone()),
                keyed_account_for_system_program(),
            ],
            &[Check::err(ProgramError::Custom(
                SystemError::ResultWithNegativeLamports as u32,
            ))],
        );
    }

    // Success.
    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (payer, payer_account.clone()),
            (recipient, recipient_account.clone()),
            keyed_account_for_system_program(),
        ],
        &[
            Check::success(),
            Check::compute_units(2345),
            Check::account(&payer)
                .lamports(payer_lamports - transfer_amount)
                .build(),
            Check::account(&recipient)
                .lamports(recipient_lamports + transfer_amount)
                .build(),
        ],
    );
}

#[test]
fn test_close_account() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();

    let mollusk = Mollusk::new(&program_id, "test_program_primary");

    let key = Pubkey::new_unique();
    let account = Account::new(50_000_000, 50, &program_id);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[3],
        vec![
            AccountMeta::new(key, true),
            AccountMeta::new(solana_sdk_ids::incinerator::id(), false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::id(), false),
        ],
    );

    // Fail account not signer.
    {
        let mut account_not_signer_ix = instruction.clone();
        account_not_signer_ix.accounts[0].is_signer = false;

        mollusk.process_and_validate_instruction(
            &account_not_signer_ix,
            &[
                (key, account.clone()),
                (solana_sdk_ids::incinerator::id(), Account::default()),
                keyed_account_for_system_program(),
            ],
            &[Check::err(ProgramError::MissingRequiredSignature)],
        );
    }

    // Success.
    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (key, account.clone()),
            (solana_sdk_ids::incinerator::id(), Account::default()),
            keyed_account_for_system_program(),
        ],
        &[
            Check::success(),
            Check::compute_units(2413),
            Check::account(&key)
                .closed() // The rest is unnecessary, just testing.
                .data(&[])
                .lamports(0)
                .owner(&solana_sdk_ids::system_program::id())
                .space(0)
                .build(),
        ],
    );
}

#[test]
fn test_cpi() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let cpi_target_program_id = Pubkey::new_unique();

    let mut mollusk = Mollusk::new(&program_id, "test_program_primary");

    let data = &[1, 2, 3, 4, 5];
    let space = data.len();
    let lamports = mollusk.sysvars.rent.minimum_balance(space);

    let key = Pubkey::new_unique();
    let account = Account::new(lamports, space, &cpi_target_program_id);

    let instruction = {
        let mut instruction_data = vec![4];
        instruction_data.extend_from_slice(cpi_target_program_id.as_ref());
        instruction_data.extend_from_slice(data);
        Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(key, true),
                AccountMeta::new_readonly(cpi_target_program_id, false),
            ],
        )
    };

    // Fail CPI target program not added to test environment.
    {
        mollusk.process_and_validate_instruction(
            &instruction,
            &[
                (key, account.clone()),
                (
                    cpi_target_program_id,
                    create_program_account_loader_v3(&cpi_target_program_id),
                ),
            ],
            &[
                // This is the error thrown by SVM. It also emits the message
                // "Program is not cached".
                Check::instruction_err(InstructionError::UnsupportedProgramId),
            ],
        );
    }

    mollusk.add_program_with_loader(
        &cpi_target_program_id,
        "test_program_cpi_target",
        &mollusk_svm::program::loader_keys::LOADER_V3,
    );

    // Fail account not signer.
    {
        let mut account_not_signer_ix = instruction.clone();
        account_not_signer_ix.accounts[0].is_signer = false;

        mollusk.process_and_validate_instruction(
            &account_not_signer_ix,
            &[
                (key, account.clone()),
                (
                    cpi_target_program_id,
                    create_program_account_loader_v3(&cpi_target_program_id),
                ),
            ],
            &[
                Check::instruction_err(InstructionError::PrivilegeEscalation), // CPI
            ],
        );
    }

    // Fail data too large.
    {
        let mut data_too_large_ix = instruction.clone();
        let mut too_large_data = vec![4];
        too_large_data.extend_from_slice(cpi_target_program_id.as_ref());
        too_large_data.extend_from_slice(&vec![1; space + 2]);
        data_too_large_ix.data = too_large_data;

        mollusk.process_and_validate_instruction(
            &data_too_large_ix,
            &[
                (key, account.clone()),
                (
                    cpi_target_program_id,
                    create_program_account_loader_v3(&cpi_target_program_id),
                ),
            ],
            &[Check::err(ProgramError::AccountDataTooSmall)],
        );
    }

    // Success.
    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (key, account.clone()),
            (
                cpi_target_program_id,
                create_program_account_loader_v3(&cpi_target_program_id),
            ),
        ],
        &[
            Check::success(),
            Check::compute_units(2130),
            Check::account(&key)
                .data(data)
                .lamports(lamports)
                .owner(&cpi_target_program_id)
                .space(space)
                .build(),
        ],
    );
}

#[test]
#[cfg(feature = "inner-instructions")]
fn test_inner_instructions_cpi() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let cpi_target_program_id = Pubkey::new_unique();

    let mut mollusk = Mollusk::new(&program_id, "test_program_primary");

    mollusk.add_program_with_loader(
        &cpi_target_program_id,
        "test_program_cpi_target",
        &mollusk_svm::program::loader_keys::LOADER_V3,
    );

    let data = &[1, 2, 3, 4, 5];
    let space = data.len();
    let lamports = mollusk.sysvars.rent.minimum_balance(space);

    let key = Pubkey::new_unique();
    let account = Account::new(lamports, space, &cpi_target_program_id);

    let instruction = {
        let mut instruction_data = vec![4];
        instruction_data.extend_from_slice(cpi_target_program_id.as_ref());
        instruction_data.extend_from_slice(data);
        Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(key, true),
                AccountMeta::new_readonly(cpi_target_program_id, false),
            ],
        )
    };

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (key, account.clone()),
            (
                cpi_target_program_id,
                create_program_account_loader_v3(&cpi_target_program_id),
            ),
        ],
        &[
            Check::success(),
            Check::inner_instruction_count(1),
            Check::account(&key)
                .data(data)
                .lamports(lamports)
                .owner(&cpi_target_program_id)
                .space(space)
                .build(),
        ],
    );

    // Use the message to map indices back to pubkeys.
    let message = result.message.as_ref().unwrap();
    let account_keys = message.account_keys();

    let inner_ix = &result.inner_instructions[0];
    assert_eq!(inner_ix.stack_height, Some(2));

    let program_id_index = inner_ix.instruction.program_id_index as usize;
    assert_eq!(
        account_keys[program_id_index], cpi_target_program_id,
        "Inner instruction program_id_index should point to the CPI target"
    );

    assert_eq!(
        inner_ix.instruction.accounts.len(),
        1,
        "Inner instruction accounts length should be 1"
    );

    let account_index = *inner_ix.instruction.accounts.first().unwrap() as usize;
    assert_eq!(
        account_keys[account_index], key,
        "Inner instruction accounts should reference the key account"
    );

    assert_eq!(
        &inner_ix.instruction.data[0], &1u8,
        "Inner instruction should be WriteData (1)"
    );
    assert_eq!(
        &inner_ix.instruction.data[1..],
        &data[1..],
        "Inner instruction data should match the CPI call"
    );
}

#[test]
#[cfg(feature = "inner-instructions")]
fn test_inner_instructions_transfer() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "test_program_primary");

    let payer = Pubkey::new_unique();
    let payer_lamports = 100_000_000;
    let payer_account = Account::new(payer_lamports, 0, &solana_sdk_ids::system_program::id());

    let recipient = Pubkey::new_unique();
    let recipient_lamports = 0;
    let recipient_account =
        Account::new(recipient_lamports, 0, &solana_sdk_ids::system_program::id());

    let transfer_amount = 2_000_000_u64;

    let transfer_instruction = {
        let mut instruction_data = vec![2];
        instruction_data.extend_from_slice(&transfer_amount.to_le_bytes());
        Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(payer, true),
                AccountMeta::new(recipient, false),
                AccountMeta::new_readonly(solana_sdk_ids::system_program::id(), false),
            ],
        )
    };

    let result = mollusk.process_and_validate_instruction(
        &transfer_instruction,
        &[
            (payer, payer_account.clone()),
            (recipient, recipient_account.clone()),
            keyed_account_for_system_program(),
        ],
        &[
            Check::success(),
            Check::inner_instruction_count(1),
            Check::account(&payer)
                .lamports(payer_lamports - transfer_amount)
                .build(),
            Check::account(&recipient)
                .lamports(recipient_lamports + transfer_amount)
                .build(),
        ],
    );

    // Use the message to map indices back to pubkeys.
    let message = result.message.as_ref().unwrap();
    let account_keys = message.account_keys();

    let inner_ix = &result.inner_instructions[0];
    assert_eq!(inner_ix.stack_height, Some(2));

    let program_id_index = inner_ix.instruction.program_id_index as usize;
    assert_eq!(
        account_keys[program_id_index],
        solana_sdk_ids::system_program::id(),
        "Inner instruction program_id_index should point to the system program"
    );

    assert_eq!(
        inner_ix.instruction.accounts.len(),
        2,
        "Inner instruction accounts length should be 2"
    );

    let payer_index = inner_ix.instruction.accounts[0] as usize;
    assert_eq!(
        account_keys[payer_index], payer,
        "Inner instruction first account should be the payer"
    );

    let recipient_index = inner_ix.instruction.accounts[1] as usize;
    assert_eq!(
        account_keys[recipient_index], recipient,
        "Inner instruction second account should be the recipient"
    );
}

#[test]
fn test_account_dedupe() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();

    let mollusk = Mollusk::new(&program_id, "test_program_primary");

    let key = Pubkey::new_unique();

    // Success first not writable.
    {
        let instruction = Instruction::new_with_bytes(
            program_id,
            &[5],
            vec![
                AccountMeta::new_readonly(key, false), // Not writable.
                AccountMeta::new_readonly(key, true),
            ],
        );
        mollusk.process_and_validate_instruction(
            &instruction,
            &[(key, Account::default()), (key, Account::default())],
            &[Check::success()],
        );
    }

    // Success second not signer.
    {
        let instruction = Instruction::new_with_bytes(
            program_id,
            &[5],
            vec![
                AccountMeta::new(key, false),
                AccountMeta::new_readonly(key, false), // Not signer.
            ],
        );
        mollusk.process_and_validate_instruction(
            &instruction,
            &[(key, Account::default()), (key, Account::default())],
            &[Check::success()],
        );
    }

    // Success with writable and signer.
    {
        let instruction = Instruction::new_with_bytes(
            program_id,
            &[5],
            vec![
                AccountMeta::new(key, false),
                AccountMeta::new_readonly(key, true),
            ],
        );
        mollusk.process_and_validate_instruction(
            &instruction,
            &[(key, Account::default()), (key, Account::default())],
            &[Check::success()],
        );
    }
}
