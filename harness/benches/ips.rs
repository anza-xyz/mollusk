//! Benches Mollusk invocation (instructions per second)
use {
    criterion::{criterion_group, criterion_main, Criterion, Throughput},
    mollusk_svm::{result::Check, Mollusk},
    solana_account::Account,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_pubkey::Pubkey,
    solana_system_program::system_processor::DEFAULT_COMPUTE_UNITS,
};

fn transfer_checked_unchecked(c: &mut Criterion) {
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let base_lamports = 100 * LAMPORTS_PER_SOL;
    let transfer_amount = 1;

    let instruction =
        solana_system_interface::instruction::transfer(&sender, &recipient, transfer_amount);
    let accounts = vec![
        (
            sender,
            Account::new(base_lamports, 0, &solana_sdk_ids::system_program::id()),
        ),
        (
            recipient,
            Account::new(base_lamports, 0, &solana_sdk_ids::system_program::id()),
        ),
    ];
    let checks = vec![
        Check::success(),
        Check::compute_units(DEFAULT_COMPUTE_UNITS),
        Check::account(&sender)
            .lamports(base_lamports - transfer_amount)
            .build(),
        Check::account(&recipient)
            .lamports(base_lamports + transfer_amount)
            .build(),
    ];

    // No logs for bench
    let mollusk = Mollusk::default();
    solana_logger::setup_with("");

    // Create transfers group with elements/second
    let mut g = c.benchmark_group("transfers");
    g.throughput(Throughput::Elements(1));

    // Bench transfer with post-execution checks
    g.bench_function("transfer_checked", |b| {
        b.iter(|| {
            mollusk.process_and_validate_instruction(&instruction, &accounts, &checks);
        })
    });

    // Bench transfer without post-execution checks
    g.bench_function("transfer_unchecked", |b| {
        b.iter(|| {
            mollusk.process_instruction(&instruction, &accounts);
        })
    });

    g.finish();
}

criterion_group!(transfers, transfer_checked_unchecked);
criterion_main!(transfers);
