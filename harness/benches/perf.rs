//! Targeted benchmarks for each identified performance bottleneck in Mollusk.
//!
//! Groups:
//!   1. `single_ix`         — Single instruction execution (baseline)
//!   2. `repeated_ix`       — Same instruction executed N times (runtime env rebuild cost)
//!   3. `chain`             — Instruction chains of varying length
//!   4. `many_accounts`     — Instructions with large account sets (account lookup cost)
//!   5. `large_account_data`— Accounts with large data buffers (clone cost)
//!   6. `transaction`       — Multi-instruction transaction execution
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mollusk_svm::Mollusk;
use solana_account::Account;
use solana_native_token::LAMPORTS_PER_SOL;
use solana_pubkey::Pubkey;


/// Helper: create a transfer instruction + accounts pair.
fn transfer_fixture() -> (solana_instruction::Instruction, Vec<(Pubkey, Account)>) {
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let base_lamports = 100 * LAMPORTS_PER_SOL;

    let instruction =
        solana_system_interface::instruction::transfer(&sender, &recipient, 1);
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
    (instruction, accounts)
}

/// Helper: create N unique transfer instructions that all share the same
/// sender but have distinct recipients, plus the full account set.
fn multi_transfer_fixtures(
    n: usize,
) -> (Vec<solana_instruction::Instruction>, Vec<(Pubkey, Account)>) {
    let sender = Pubkey::new_unique();
    let base_lamports = 1_000 * LAMPORTS_PER_SOL;

    let mut accounts = vec![(
        sender,
        Account::new(base_lamports, 0, &solana_sdk_ids::system_program::id()),
    )];
    let mut instructions = Vec::with_capacity(n);

    for _ in 0..n {
        let recipient = Pubkey::new_unique();
        instructions.push(solana_system_interface::instruction::transfer(
            &sender, &recipient, 1,
        ));
        accounts.push((
            recipient,
            Account::new(base_lamports, 0, &solana_sdk_ids::system_program::id()),
        ));
    }

    (instructions, accounts)
}

// ── 1. Single instruction baseline ──────────────────────────────────────

fn bench_single_instruction(c: &mut Criterion) {
    let mollusk = Mollusk::default();
    solana_logger::setup_with("");
    let (instruction, accounts) = transfer_fixture();

    let mut g = c.benchmark_group("single_ix");
    g.throughput(Throughput::Elements(1));

    g.bench_function("transfer", |b| {
        b.iter(|| {
            mollusk.process_instruction(&instruction, &accounts);
        })
    });

    g.finish();
}

// ── 2. Repeated single-instruction execution ────────────────────────────
// Measures overhead of rebuilding runtime environments per call.

fn bench_repeated_instructions(c: &mut Criterion) {
    let mollusk = Mollusk::default();
    solana_logger::setup_with("");
    let (instruction, accounts) = transfer_fixture();

    let mut g = c.benchmark_group("repeated_ix");

    for count in [10, 50, 100] {
        g.throughput(Throughput::Elements(count as u64));
        g.bench_with_input(
            BenchmarkId::new("transfers", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    for _ in 0..count {
                        mollusk.process_instruction(&instruction, &accounts);
                    }
                })
            },
        );
    }

    g.finish();
}

// ── 3. Instruction chains ───────────────────────────────────────────────
// Each instruction in the chain creates a new transaction context.

fn bench_instruction_chain(c: &mut Criterion) {
    let mollusk = Mollusk::default();
    solana_logger::setup_with("");

    let mut g = c.benchmark_group("chain");

    for chain_len in [2, 5, 10, 20] {
        let (instructions, accounts) = multi_transfer_fixtures(chain_len);
        g.throughput(Throughput::Elements(chain_len as u64));
        g.bench_with_input(
            BenchmarkId::new("len", chain_len),
            &(instructions, accounts),
            |b, (ixs, accs)| {
                b.iter(|| {
                    mollusk.process_instruction_chain(ixs, accs);
                })
            },
        );
    }

    g.finish();
}

// ── 4. Many accounts (account lookup stress) ────────────────────────────
// The current code does O(n) linear scans per account key. This benchmark
// creates instructions with progressively more "bystander" accounts so
// we can measure the lookup overhead.

fn bench_many_accounts(c: &mut Criterion) {
    let mollusk = Mollusk::default();
    solana_logger::setup_with("");

    let mut g = c.benchmark_group("many_accounts");

    for extra_accounts in [0, 10, 50, 100, 200] {
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let base_lamports = 100 * LAMPORTS_PER_SOL;

        let instruction =
            solana_system_interface::instruction::transfer(&sender, &recipient, 1);
        let mut accounts = vec![
            (
                sender,
                Account::new(base_lamports, 0, &solana_sdk_ids::system_program::id()),
            ),
            (
                recipient,
                Account::new(base_lamports, 0, &solana_sdk_ids::system_program::id()),
            ),
        ];

        // Add extra accounts that are provided but not part of the instruction.
        // These make the accounts slice larger, stressing the linear lookup.
        for _ in 0..extra_accounts {
            accounts.push((
                Pubkey::new_unique(),
                Account::new(
                    LAMPORTS_PER_SOL,
                    0,
                    &solana_sdk_ids::system_program::id(),
                ),
            ));
        }

        let total = 2 + extra_accounts;
        g.throughput(Throughput::Elements(1));
        g.bench_with_input(
            BenchmarkId::new("total_accounts", total),
            &(instruction.clone(), accounts),
            |b, (ix, accs)| {
                b.iter(|| {
                    mollusk.process_instruction(ix, accs);
                })
            },
        );
    }

    g.finish();
}

// ── 5. Large account data (clone cost) ──────────────────────────────────
// Measures the cost of deep-cloning accounts with large data buffers.

fn bench_large_account_data(c: &mut Criterion) {
    let mollusk = Mollusk::default();
    solana_logger::setup_with("");

    let mut g = c.benchmark_group("large_account_data");

    for data_size in [0, 1024, 10_240, 102_400] {
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let base_lamports = 100 * LAMPORTS_PER_SOL;

        let instruction =
            solana_system_interface::instruction::transfer(&sender, &recipient, 1);

        // Sender has a large data buffer to stress cloning.
        let mut sender_account =
            Account::new(base_lamports, data_size, &solana_sdk_ids::system_program::id());
        // Fill with non-zero data so the allocator actually has to copy it.
        sender_account.data.fill(0xAB);

        let accounts = vec![
            (sender, sender_account),
            (
                recipient,
                Account::new(base_lamports, 0, &solana_sdk_ids::system_program::id()),
            ),
        ];

        g.throughput(Throughput::Elements(1));
        g.bench_with_input(
            BenchmarkId::new("data_bytes", data_size),
            &(instruction.clone(), accounts),
            |b, (ix, accs)| {
                b.iter(|| {
                    mollusk.process_instruction(ix, accs);
                })
            },
        );
    }

    g.finish();
}

// ── 6. Multi-instruction transaction (shared context) ───────────────────
// Uses process_transaction_instructions which shares a single transaction
// context across all instructions.

fn bench_transaction(c: &mut Criterion) {
    let mollusk = Mollusk::default();
    solana_logger::setup_with("");

    let mut g = c.benchmark_group("transaction");

    for n_instructions in [2, 5, 10, 20] {
        let (instructions, accounts) = multi_transfer_fixtures(n_instructions);
        g.throughput(Throughput::Elements(n_instructions as u64));
        g.bench_with_input(
            BenchmarkId::new("instructions", n_instructions),
            &(instructions, accounts),
            |b, (ixs, accs)| {
                b.iter(|| {
                    mollusk.process_transaction_instructions(ixs, accs);
                })
            },
        );
    }

    g.finish();
}

criterion_group!(
    benches,
    bench_single_instruction,
    bench_repeated_instructions,
    bench_instruction_chain,
    bench_many_accounts,
    bench_large_account_data,
    bench_transaction,
);
criterion_main!(benches);
