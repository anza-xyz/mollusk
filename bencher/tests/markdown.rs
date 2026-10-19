use {
    mollusk_svm::Mollusk,
    mollusk_svm_bencher::{MolluskComputeUnitBencher, MolluskComputeUnitMatrixBencher},
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
};

#[test]
fn test_markdown() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");
    solana_logger::setup_with("");

    let program_id = Pubkey::new_unique();

    let instruction = Instruction::new_with_bytes(program_id, &[0], vec![]);
    let accounts = vec![];

    let mollusk = Mollusk::new(&program_id, "test_program_primary");

    MolluskComputeUnitBencher::new(mollusk)
        .bench(("bench0", &instruction, &accounts))
        .bench(("bench1", &instruction, &accounts))
        .bench(("bench2", &instruction, &accounts))
        .bench(("bench3", &instruction, &accounts))
        .bench(("bench4", &instruction, &accounts))
        .bench(("bench5", &instruction, &accounts))
        .bench(("bench6", &instruction, &accounts))
        .must_pass(true)
        .out_dir("../target/benches")
        .execute();
}

#[cfg(feature = "cpi-compute-units")]
#[test]
fn test_markdown_cpi_breakdown() {
    use {
        mollusk_svm::program::{create_program_account_loader_v3, loader_keys::LOADER_V3},
        solana_account::Account,
        solana_instruction::AccountMeta,
    };

    std::env::set_var("SBF_OUT_DIR", "../target/deploy");
    solana_logger::setup_with("");

    let program_id = Pubkey::new_unique();
    let cpi_target_program_id = Pubkey::new_unique();
    let key = Pubkey::new_unique();

    let new_mollusk = || {
        let mut mollusk = Mollusk::new(&program_id, "test_program_primary");
        mollusk.add_program_with_loader(
            &cpi_target_program_id,
            "test_program_cpi_target",
            &LOADER_V3,
        );
        mollusk
    };

    let noop_instruction = Instruction::new_with_bytes(program_id, &[0], vec![]);
    let cpi_instruction = {
        let mut data = vec![4];
        data.extend_from_slice(cpi_target_program_id.as_ref());
        data.extend_from_slice(&[1, 2, 3, 4, 5]);
        Instruction::new_with_bytes(
            program_id,
            &data,
            vec![
                AccountMeta::new(key, true),
                AccountMeta::new_readonly(cpi_target_program_id, false),
            ],
        )
    };
    let no_accounts = vec![];
    let cpi_accounts = {
        let mollusk = new_mollusk();
        vec![
            (
                key,
                Account::new(
                    mollusk.sysvars.rent.minimum_balance(5),
                    5,
                    &cpi_target_program_id,
                ),
            ),
            (
                cpi_target_program_id,
                create_program_account_loader_v3(&cpi_target_program_id),
            ),
        ]
    };

    let (noop_cus, cpi_cus, cpi_cpi_cus) = {
        let mollusk = new_mollusk();
        let noop = mollusk.process_instruction(&noop_instruction, &no_accounts);
        let cpi = mollusk.process_instruction(&cpi_instruction, &cpi_accounts);
        (
            noop.compute_units_consumed,
            cpi.compute_units_consumed,
            cpi.cpi_compute_units[0].compute_units_consumed,
        )
    };

    let out_dir = std::env::temp_dir().join(format!("mollusk-bencher-{}", Pubkey::new_unique()));
    std::fs::create_dir_all(&out_dir).unwrap();
    let path = out_dir.join("compute_units.md");
    std::fs::write(
        &path,
        format!(
            r#"#### previous

Solana CLI Version: Unknown

| Name | CUs | Delta |
|------|------|-------|
| noop | {noop_cus} | - new - |
| cpi | {cpi_cus} | - new - |

"#
        ),
    )
    .unwrap();

    let run = || {
        MolluskComputeUnitBencher::new(new_mollusk())
            .bench(("noop", &noop_instruction, &no_accounts))
            .bench(("cpi", &cpi_instruction, &cpi_accounts))
            .must_pass(true)
            .cpi_breakdown(true)
            .out_dir(out_dir.to_str().unwrap())
            .execute();
    };

    run();
    let content = std::fs::read_to_string(&path).unwrap();
    let table = content.lines().skip(4).take(4).collect::<Vec<_>>();
    assert_eq!(
        table,
        [
            "| Name | CUs | Delta | Program | CPIs |".to_string(),
            "|------|------|-------|---------|------|".to_string(),
            format!("| noop | {noop_cus} | -- | {noop_cus} | 0 |"),
            format!(
                "| cpi | {cpi_cus} | -- | {} | {cpi_cpi_cus} |",
                cpi_cus - cpi_cpi_cus
            ),
        ]
    );

    run();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), content);

    std::fs::remove_dir_all(&out_dir).unwrap();
}

#[test]
fn mx_test_markdown() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");
    solana_logger::setup_with("");

    let program_id = Pubkey::new_unique();

    let instruction = Instruction::new_with_bytes(program_id, &[0], vec![]);
    let accounts = vec![];

    let mut mollusk = Mollusk::new(&program_id, "test_program_primary");

    MolluskComputeUnitMatrixBencher::new(&mut mollusk)
        .programs(&["test_program_primary", "test_program_noop_log"])
        .bench(("bench0", &instruction, &accounts))
        .bench(("bench1", &instruction, &accounts))
        .bench(("bench2", &instruction, &accounts))
        .bench(("bench3", &instruction, &accounts))
        .bench(("bench4", &instruction, &accounts))
        .bench(("bench5", &instruction, &accounts))
        .bench(("bench6", &instruction, &accounts))
        .must_pass(true)
        .out_dir("../target/benches")
        .execute();
}
