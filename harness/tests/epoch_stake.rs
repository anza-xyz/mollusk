use {
    mollusk_svm::{result::Check, Mollusk},
    solana_account::Account,
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
};

#[test]
fn test_epoch_stake() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let mut mollusk = Mollusk::new(&program_id, "test_program_epoch_stake");

    let key = Pubkey::new_unique();

    let mut total_stake: u64 = 0;

    for i in 1..=3 {
        let stake = 1_000_000_000_000_000 * i;
        total_stake += stake;

        let vote_address = Pubkey::new_unique();

        mollusk.epoch_stake.insert(vote_address, stake);
        assert_eq!(total_stake, mollusk.epoch_stake.values().sum::<u64>());

        let instruction = Instruction::new_with_bytes(
            program_id,
            &vote_address.to_bytes(),
            vec![AccountMeta::new(key, false)],
        );

        mollusk.process_and_validate_instruction(
            &instruction,
            &[(key, Account::new(1_000, 16, &program_id))],
            &[
                Check::success(),
                Check::account(&key)
                    .data(&{
                        let mut data = vec![0; 16];
                        data[0..8].copy_from_slice(&total_stake.to_le_bytes());
                        data[8..16].copy_from_slice(&stake.to_le_bytes());
                        data
                    })
                    .build(),
            ],
        );
    }
}
