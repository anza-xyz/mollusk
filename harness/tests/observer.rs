use {
    mollusk_svm::{result::Check, Mollusk},
    solana_account::{Account, ReadableAccount},
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
    solana_svm_log_collector::LogCollector,
    std::cell::RefCell,
    std::rc::Rc,
};

#[test]
fn test_observer_accesses_result_logs_and_context() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mut mollusk = Mollusk::new(&program_id, "test_program_primary");
    // Provide a log collector so logs are available via the context.
    mollusk.logger = Some(Rc::new(RefCell::new(LogCollector::default())));

    // A simple write-data instruction that succeeds.
    let data = vec![9u8, 8, 7, 6];
    let space = data.len();
    let lamports = mollusk.sysvars.rent.minimum_balance(space);

    let key = Pubkey::new_unique();
    let account = Account::new(lamports, space, &program_id);

    let instruction = {
        let mut instruction_data = vec![1];
        instruction_data.extend_from_slice(&data);
        Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![AccountMeta::new(key, true)],
        )
    };

    // Captured state from the observer.
    let observed_compute_units = Rc::new(RefCell::new(0u64));
    let observed_owner = Rc::new(RefCell::new(None::<Pubkey>));
    let observed_data_len = Rc::new(RefCell::new(0usize));
    let observed_account_data = Rc::new(RefCell::new(Vec::<u8>::new()));
    let observed_logs_match = Rc::new(RefCell::new(false));

    let cu_ref = observed_compute_units.clone();
    let owner_ref = observed_owner.clone();
    let data_len_ref = observed_data_len.clone();
    let data_ref = observed_account_data.clone();
    let logs_ok_ref = observed_logs_match.clone();

    let result = mollusk.process_and_validate_instruction_with_observer(
        &instruction,
        &[(key, account.clone())],
        &[Check::success()],
        move |_inst, res, ctx| {
            // Access compute units from the result.
            *cu_ref.borrow_mut() = res.compute_units_consumed;

            // Access full context to inspect accounts.
            if let Some(index) = ctx.transaction_context.find_index_of_account(&key) {
                let acc = ctx
                    .transaction_context
                    .accounts()
                    .try_borrow(index)
                    .unwrap()
                    .clone();
                *owner_ref.borrow_mut() = Some(*acc.owner());
                let data_vec = acc.data().to_vec();
                *data_len_ref.borrow_mut() = data_vec.len();
                *data_ref.borrow_mut() = data_vec;
            }

            // Access and validate logs via the context's log collector.
            if let Some(logs_rc) = ctx.get_log_collector() {
                let logs_dbg = logs_rc.borrow().get_recorded_content().join("\n");
                let pid = program_id.to_string();
                let ok = logs_dbg.contains(&format!("Program {} invoke", pid))
                    && logs_dbg.contains("consumed 384")
                    && logs_dbg.contains(&format!("Program {} success", pid));
                *logs_ok_ref.borrow_mut() = ok;
            }
        },
    );

    // Normal checks still pass.
    assert!(result.program_result.is_ok());

    // The observer should have captured compute units and account state.
    assert_eq!(*observed_compute_units.borrow(), 384);
    assert_eq!(observed_owner.borrow().unwrap(), program_id);
    assert_eq!(*observed_data_len.borrow(), space);
    assert_eq!(&*observed_account_data.borrow(), &data);
    assert!(*observed_logs_match.borrow());
}
