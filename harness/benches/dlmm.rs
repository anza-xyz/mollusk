//! Benchmark for Meteora DLMM swap simulation using real on-chain state.
//!
//! Loads a pre-baked fixture (instruction + accounts) generated from anvil2's
//! frozen snapshot. This represents a realistic real-world Mollusk workload:
//! a BPF program swap with ~19 accounts, CPI into SPL Token, and large
//! account data buffers.
//!
//! Fixture generation (run from anvil2):
//!   cargo test -p dex --test meteora_dlmm -- dump_bench_fixture --ignored

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use mollusk_svm::Mollusk;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use std::str::FromStr;

// ── Constants ───────────────────────────────────────────────────────────

const DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const MEMO_PROGRAM: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
const UPGRADEABLE_LOADER: &str = "BPFLoaderUpgradeab1e11111111111111111111111";

// Paths to anvil2 fixtures.
const ANVIL2_ROOT: &str = concat!(env!("HOME"), "/Git/anvil2/members/dex");

// ── Fixture loading ─────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct FixtureJson {
    instruction: InstructionJson,
    accounts: Vec<AccountJson>,
    quote_context: QuoteContextJson,
}

#[derive(serde::Deserialize)]
struct InstructionJson {
    program_id: String,
    data: String,
    account_metas: Vec<AccountMetaJson>,
}

#[derive(serde::Deserialize)]
struct AccountMetaJson {
    pubkey: String,
    is_signer: bool,
    is_writable: bool,
}

#[derive(serde::Deserialize)]
struct AccountJson {
    pubkey: String,
    lamports: u64,
    owner: String,
    executable: bool,
    rent_epoch: u64,
    data: String,
}

#[derive(serde::Deserialize)]
struct QuoteContextJson {
    unix_timestamp: u64,
    slot: u64,
    epoch: u64,
}

struct Fixture {
    instruction: Instruction,
    accounts: Vec<(Pubkey, Account)>,
    slot: u64,
    epoch: u64,
    unix_timestamp: i64,
}

fn load_fixture() -> Fixture {
    use base64::Engine as _;

    let path = format!("{ANVIL2_ROOT}/tests/meteora_dlmm/bench_fixture.json");
    let json: FixtureJson = serde_json::from_str(
        &std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Missing {path}: {e}")),
    )
    .unwrap();

    let instruction = Instruction {
        program_id: Pubkey::from_str(&json.instruction.program_id).unwrap(),
        data: base64::engine::general_purpose::STANDARD
            .decode(&json.instruction.data)
            .unwrap(),
        accounts: json
            .instruction
            .account_metas
            .iter()
            .map(|m| {
                let pubkey = Pubkey::from_str(&m.pubkey).unwrap();
                if m.is_writable {
                    AccountMeta::new(pubkey, m.is_signer)
                } else {
                    AccountMeta::new_readonly(pubkey, m.is_signer)
                }
            })
            .collect(),
    };

    let accounts = json
        .accounts
        .iter()
        .map(|a| {
            (
                Pubkey::from_str(&a.pubkey).unwrap(),
                Account {
                    lamports: a.lamports,
                    data: base64::engine::general_purpose::STANDARD
                        .decode(&a.data)
                        .unwrap(),
                    owner: Pubkey::from_str(&a.owner).unwrap(),
                    executable: a.executable,
                    rent_epoch: a.rent_epoch,
                },
            )
        })
        .collect();

    Fixture {
        instruction,
        accounts,
        slot: json.quote_context.slot,
        epoch: json.quote_context.epoch,
        unix_timestamp: json.quote_context.unix_timestamp as i64,
    }
}

fn load_elf(relative: &str) -> Vec<u8> {
    let path = format!("{ANVIL2_ROOT}/{relative}");
    std::fs::read(&path).unwrap_or_else(|e| panic!("Missing {path}: {e}"))
}

// ── Setup Mollusk ───────────────────────────────────────────────────────

fn setup_mollusk(fixture: &Fixture) -> Mollusk {
    let loader = Pubkey::from_str(UPGRADEABLE_LOADER).unwrap();
    let mut mollusk = Mollusk::default();

    // Configure clock
    mollusk.sysvars.warp_to_slot(fixture.slot);
    mollusk.sysvars.clock.epoch = fixture.epoch;
    mollusk.sysvars.clock.leader_schedule_epoch = fixture.epoch;
    mollusk.sysvars.clock.unix_timestamp = fixture.unix_timestamp;
    mollusk.rebuild_sysvar_cache();

    // Load programs
    let programs: &[(&str, &str)] = &[
        (DLMM_PROGRAM_ID, "tests/fixtures/meteora_dlmm.so"),
        (TOKEN_PROGRAM, "tests/fixtures/spl_token.so"),
        (TOKEN_2022_PROGRAM, "tests/fixtures/spl_token_2022.so"),
        (MEMO_PROGRAM, "tests/fixtures/spl_memo.so"),
    ];
    for (id, elf_path) in programs {
        let pubkey = Pubkey::from_str(id).unwrap();
        let elf = load_elf(elf_path);
        mollusk.add_program_with_loader_and_elf(&pubkey, &loader, &elf);
    }

    solana_logger::setup_with("");
    mollusk
}

// ── Benchmarks ──────────────────────────────────────────────────────────

fn bench_dlmm_swap(c: &mut Criterion) {
    let fixture = load_fixture();
    let mollusk = setup_mollusk(&fixture);

    // Verify it works
    let result = mollusk.process_instruction(&fixture.instruction, &fixture.accounts);
    assert!(
        result.program_result.is_ok(),
        "DLMM swap failed: {:?}",
        result.program_result
    );
    println!(
        "DLMM swap: {} CUs, {} accounts, {} ix bytes",
        result.compute_units_consumed,
        fixture.accounts.len(),
        fixture.instruction.data.len(),
    );

    let mut g = c.benchmark_group("dlmm");

    g.throughput(Throughput::Elements(1));
    g.bench_function("swap", |b| {
        b.iter(|| {
            mollusk.process_instruction(&fixture.instruction, &fixture.accounts);
        })
    });

    g.throughput(Throughput::Elements(10));
    g.bench_function("swap_x10", |b| {
        b.iter(|| {
            for _ in 0..10 {
                mollusk.process_instruction(&fixture.instruction, &fixture.accounts);
            }
        })
    });

    g.finish();
}

criterion_group!(benches, bench_dlmm_swap);
criterion_main!(benches);
