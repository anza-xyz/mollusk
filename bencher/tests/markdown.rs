use {
    mollusk_svm::Mollusk, mollusk_svm_bencher::MolluskComputeUnitBencher,
    solana_instruction::Instruction, solana_pubkey::Pubkey,
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
