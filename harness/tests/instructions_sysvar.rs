use {
    mollusk_svm::{result::Check, Mollusk},
    solana_account::Account,
    solana_instruction::{AccountMeta, BorrowedAccountMeta, BorrowedInstruction, Instruction},
    solana_instructions_sysvar::construct_instructions_data,
    solana_program_error::ProgramError,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
};

const ENTRY_SIZE: usize = 35; // program_id (32) + instruction_index (2) + executed (1)

fn parse_entry(data: &[u8], index: usize) -> (Pubkey, u16, bool) {
    let offset = index * ENTRY_SIZE;
    let program_id = Pubkey::new_from_array(data[offset..offset + 32].try_into().unwrap());
    let instruction_index = u16::from_le_bytes(data[offset + 32..offset + 34].try_into().unwrap());
    let executed = data[offset + 34] == 1;
    (program_id, instruction_index, executed)
}

fn as_borrowed_instruction(instruction: &Instruction) -> BorrowedInstruction<'_> {
    BorrowedInstruction {
        program_id: &instruction.program_id,
        accounts: instruction
            .accounts
            .iter()
            .map(|meta| BorrowedAccountMeta {
                pubkey: &meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: &instruction.data,
    }
}

#[test]
fn test_single_instruction() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "test_program_instructions_sysvar");

    let output_pubkey = Pubkey::new_unique();
    let output_is_signer = false;
    let output_is_writable = true;
    let output_space = ENTRY_SIZE; // 1 instruction entry.
    let output_lamports = Rent::default().minimum_balance(output_space);

    let extra_pubkey1 = Pubkey::new_unique();
    let extra_is_signer1 = true;
    let extra_is_writable1 = false;

    let extra_pubkey2 = Pubkey::new_unique();
    let extra_is_signer2 = false;
    let extra_is_writable2 = true;

    let input_data = &[
        output_is_signer as u8,
        output_is_writable as u8,
        extra_is_signer1 as u8,
        extra_is_writable1 as u8,
        extra_is_signer2 as u8,
        extra_is_writable2 as u8,
        // Skip the ix sysvar account.
    ];

    let instruction = Instruction::new_with_bytes(
        program_id,
        input_data,
        vec![
            AccountMeta {
                pubkey: output_pubkey,
                is_signer: output_is_signer,
                is_writable: output_is_writable,
            },
            AccountMeta {
                pubkey: extra_pubkey1,
                is_signer: extra_is_signer1,
                is_writable: extra_is_writable1,
            },
            AccountMeta {
                pubkey: extra_pubkey2,
                is_signer: extra_is_signer2,
                is_writable: extra_is_writable2,
            },
            AccountMeta::new_readonly(solana_instructions_sysvar::ID, false),
        ],
    );

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (
                output_pubkey,
                Account::new(output_lamports, ENTRY_SIZE, &program_id),
            ),
            (extra_pubkey1, Account::default()),
            (extra_pubkey2, Account::default()),
            // Don't provide an account for ix sysvar.
        ],
        &[Check::success()],
    );

    // Verify an entry was written for the instruction.
    let output_account = result.get_account(&output_pubkey).unwrap();
    let (written_program_id, written_index, executed) = parse_entry(&output_account.data, 0);
    assert_eq!(written_program_id, program_id);
    assert_eq!(written_index, 0);
    assert!(!executed);

    // Since no account was provided for the ix sysvar, it should not be returned.
    assert!(result
        .get_account(&solana_instructions_sysvar::ID)
        .is_none());
}

#[test]
fn test_instruction_chain() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "test_program_instructions_sysvar");

    let output_pubkey = Pubkey::new_unique();
    let output_is_signer = false;
    let output_is_writable = true;
    let output_space = 3 * ENTRY_SIZE; // 3 instruction entries.
    let output_lamports = Rent::default().minimum_balance(output_space);

    let build_instruction = |extra_accounts: &[(bool, bool)]| -> (Instruction, Vec<Pubkey>) {
        let extra_pubkeys: Vec<Pubkey> = extra_accounts
            .iter()
            .map(|_| Pubkey::new_unique())
            .collect();

        let mut input_data = vec![output_is_signer as u8, output_is_writable as u8];
        for &(is_signer, is_writable) in extra_accounts {
            input_data.push(is_signer as u8);
            input_data.push(is_writable as u8);
        }
        // Skip the ix sysvar account.

        let mut account_metas = vec![AccountMeta {
            pubkey: output_pubkey,
            is_signer: output_is_signer,
            is_writable: output_is_writable,
        }];
        for (pubkey, &(is_signer, is_writable)) in extra_pubkeys.iter().zip(extra_accounts) {
            account_metas.push(AccountMeta {
                pubkey: *pubkey,
                is_signer,
                is_writable,
            });
        }
        account_metas.push(AccountMeta::new_readonly(
            solana_instructions_sysvar::ID,
            false,
        ));

        (
            Instruction::new_with_bytes(program_id, &input_data, account_metas),
            extra_pubkeys,
        )
    };

    let (instruction1, extra_pubkeys1) = build_instruction(&[(true, false), (false, true)]);
    let (instruction2, extra_pubkeys2) = build_instruction(&[(true, true)]);
    let (instruction3, extra_pubkeys3) =
        build_instruction(&[(false, false), (true, true), (false, true)]);

    let extra_accounts: Vec<(Pubkey, Account)> = [extra_pubkeys1, extra_pubkeys2, extra_pubkeys3]
        .into_iter()
        .flatten()
        .map(|pubkey| (pubkey, Account::default()))
        .collect();

    let mut accounts = vec![(
        output_pubkey,
        Account::new(output_lamports, output_space, &program_id),
    )];
    accounts.extend(extra_accounts);

    let result = mollusk.process_and_validate_instruction_chain(
        &[
            (&instruction1, &[Check::success()]),
            (&instruction2, &[Check::success()]),
            (&instruction3, &[Check::success()]),
        ],
        &accounts,
    );

    // Verify entries were written for each instruction.
    let output_account = result.get_account(&output_pubkey).unwrap();

    // Entry 0: index=0, executed=true (marked by instruction 1)
    let (pid0, idx0, exec0) = parse_entry(&output_account.data, 0);
    assert_eq!(pid0, program_id);
    assert_eq!(idx0, 0);
    assert!(exec0);

    // Entry 1: index=1, executed=true (marked by instruction 2)
    let (pid1, idx1, exec1) = parse_entry(&output_account.data, 1);
    assert_eq!(pid1, program_id);
    assert_eq!(idx1, 1);
    assert!(exec1);

    // Entry 2: index=2, executed=false (current/last instruction)
    let (pid2, idx2, exec2) = parse_entry(&output_account.data, 2);
    assert_eq!(pid2, program_id);
    assert_eq!(idx2, 2);
    assert!(!exec2); // Last instruction not marked as executed.

    // Since no account was provided for the ix sysvar, it should not be returned.
    assert!(result
        .get_account(&solana_instructions_sysvar::ID)
        .is_none());
}

#[test]
fn test_override_sysvar_arbitrary() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "test_program_instructions_sysvar");

    let output_key = Pubkey::new_unique();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[],
        vec![
            AccountMeta::new(output_key, false),
            AccountMeta::new_readonly(solana_instructions_sysvar::ID, false),
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (output_key, Account::default()),
            (
                solana_instructions_sysvar::ID,
                Account::default(), // Use default here.
            ),
        ],
        &[
            // Since we provided `Account::default()` for the sysvar, the
            // program should error when validating its owner.
            Check::err(ProgramError::InvalidAccountOwner),
        ],
    );
}

#[test]
fn test_override_sysvar_actual() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mollusk = Mollusk::new(&program_id, "test_program_instructions_sysvar");

    let output_pubkey = Pubkey::new_unique();
    let output_is_signer = false;
    let output_is_writable = true;
    let output_space = ENTRY_SIZE; // 1 instruction entry.
    let output_lamports = Rent::default().minimum_balance(output_space);

    let extra_pubkey1 = Pubkey::new_unique();
    let extra_is_signer1 = true;
    let extra_is_writable1 = false;

    let input_data = &[
        output_is_signer as u8,
        output_is_writable as u8,
        extra_is_signer1 as u8,
        extra_is_writable1 as u8,
        // Skip the ix sysvar account.
    ];

    let mut instruction = Instruction::new_with_bytes(
        program_id,
        input_data,
        vec![
            AccountMeta {
                pubkey: output_pubkey,
                is_signer: output_is_signer,
                is_writable: output_is_writable,
            },
            AccountMeta {
                pubkey: extra_pubkey1,
                is_signer: extra_is_signer1,
                is_writable: extra_is_writable1,
            },
            AccountMeta::new_readonly(solana_instructions_sysvar::ID, false),
        ],
    );

    // Mock out the sysvar account and provide it to the test environment.
    let ix_sysvar_account_data =
        construct_instructions_data(&[as_borrowed_instruction(&instruction)]);
    let mut ix_sysvar_account =
        Account::new(0, ix_sysvar_account_data.len(), &solana_sdk_ids::sysvar::ID);
    ix_sysvar_account.data = ix_sysvar_account_data.clone();

    // Now intentionally flip one of the is_writable flags in the instruction
    // so that we can verify the sysvar data was actually used.
    instruction.accounts[1].is_writable = !extra_is_writable1;

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (
                output_pubkey,
                Account::new(output_lamports, ENTRY_SIZE, &program_id),
            ),
            (extra_pubkey1, Account::default()),
            (solana_instructions_sysvar::ID, ix_sysvar_account),
        ],
        &[Check::success()],
    );

    // Verify an entry was written for the instruction.
    let output_account = result.get_account(&output_pubkey).unwrap();
    let (written_program_id, written_index, executed) = parse_entry(&output_account.data, 0);
    assert_eq!(written_program_id, program_id);
    assert_eq!(written_index, 0);
    assert!(!executed);

    // Since the account was provided for the ix sysvar, it should be returned.
    // It should also be unchanged.
    let resulting_ix_sysvar_account = result.get_account(&solana_instructions_sysvar::ID).unwrap();
    assert_eq!(resulting_ix_sysvar_account.data, ix_sysvar_account_data);
}
