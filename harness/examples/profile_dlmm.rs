//! Profiles the DLMM swap execution to show where wall-clock time is spent.
//!
//! Uses `InstructionResult::execution_time` (SVM's own BPF timing) to split
//! harness overhead from actual program execution.
//!
//! Run with:
//!   cargo run --example profile_dlmm --release

use mollusk_svm::Mollusk;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use std::str::FromStr;
use std::time::Instant;

const ANVIL2_ROOT: &str = concat!(env!("HOME"), "/Git/anvil2/members/dex");
const ITERATIONS: usize = 2000;

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

fn load_fixture() -> (Instruction, Vec<(Pubkey, Account)>, u64, u64, i64) {
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

    (
        instruction,
        accounts,
        json.quote_context.slot,
        json.quote_context.epoch,
        json.quote_context.unix_timestamp as i64,
    )
}

fn load_elf(relative: &str) -> Vec<u8> {
    let path = format!("{ANVIL2_ROOT}/{relative}");
    std::fs::read(&path).unwrap_or_else(|e| panic!("Missing {path}: {e}"))
}

fn main() {
    let (instruction, accounts, slot, epoch, unix_timestamp) = load_fixture();

    let loader = Pubkey::from_str("BPFLoaderUpgradeab1e11111111111111111111111").unwrap();
    let mut mollusk = Mollusk::default();

    mollusk.sysvars.warp_to_slot(slot);
    mollusk.sysvars.clock.epoch = epoch;
    mollusk.sysvars.clock.leader_schedule_epoch = epoch;
    mollusk.sysvars.clock.unix_timestamp = unix_timestamp;
    mollusk.rebuild_sysvar_cache();

    for (id, elf) in [
        ("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo", "tests/fixtures/meteora_dlmm.so"),
        ("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", "tests/fixtures/spl_token.so"),
        ("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb", "tests/fixtures/spl_token_2022.so"),
        ("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr", "tests/fixtures/spl_memo.so"),
    ] {
        mollusk.add_program_with_loader_and_elf(
            &Pubkey::from_str(id).unwrap(),
            &loader,
            &load_elf(elf),
        );
    }

    solana_logger::setup_with("");

    // Warmup
    for _ in 0..50 {
        let _ = mollusk.process_instruction(&instruction, &accounts);
    }

    // Collect per-iteration data
    let mut wall_times = Vec::with_capacity(ITERATIONS);
    let mut svm_exec_times = Vec::with_capacity(ITERATIONS);
    let mut compute_units = Vec::with_capacity(ITERATIONS);

    for _ in 0..ITERATIONS {
        let start = Instant::now();
        let result = mollusk.process_instruction(&instruction, &accounts);
        let wall = start.elapsed();

        assert!(result.program_result.is_ok());
        wall_times.push(wall.as_nanos() as u64);
        svm_exec_times.push(result.execution_time); // in microseconds from SVM
        compute_units.push(result.compute_units_consumed);
    }

    // Statistics
    wall_times.sort();
    svm_exec_times.sort();

    let wall_median = wall_times[ITERATIONS / 2];
    let wall_p5 = wall_times[ITERATIONS * 5 / 100];
    let wall_p95 = wall_times[ITERATIONS * 95 / 100];
    let wall_mean: u64 = wall_times.iter().sum::<u64>() / ITERATIONS as u64;

    let svm_median = svm_exec_times[ITERATIONS / 2];
    let svm_mean: u64 = svm_exec_times.iter().sum::<u64>() / ITERATIONS as u64;

    let harness_median = wall_median.saturating_sub(svm_median * 1000); // svm is in µs, wall in ns
    let harness_mean = wall_mean.saturating_sub(svm_mean * 1000);

    let cu = compute_units[0];

    // Account data analysis
    let total_data: usize = accounts.iter().map(|(_, a)| a.data.len()).sum();
    let max_data = accounts.iter().map(|(_, a)| a.data.len()).max().unwrap_or(0);
    let writable_count = instruction.accounts.iter().filter(|m| m.is_writable).count();

    println!("=== DLMM Swap Profile ({ITERATIONS} iterations, release mode) ===");
    println!();
    println!("Instruction: {} CUs, {} bytes data", cu, instruction.data.len());
    println!("Accounts:    {} total, {} writable", accounts.len(), writable_count);
    println!("Data:        {} KB total, {} KB largest",
        total_data / 1024, max_data / 1024);
    println!();
    println!("--- Timing Breakdown ---");
    println!();
    println!("                      median      mean        p5         p95");
    println!("Wall clock:       {:>8.1} µs  {:>7.1} µs  {:>7.1} µs  {:>7.1} µs",
        wall_median as f64 / 1000.0,
        wall_mean as f64 / 1000.0,
        wall_p5 as f64 / 1000.0,
        wall_p95 as f64 / 1000.0,
    );
    println!("SVM execution:    {:>8.1} µs  {:>7.1} µs",
        svm_median as f64,
        svm_mean as f64,
    );
    println!("Harness overhead: {:>8.1} µs  {:>7.1} µs",
        harness_median as f64 / 1000.0,
        harness_mean as f64 / 1000.0,
    );
    println!();
    println!("--- Split ---");
    println!();
    let svm_pct = svm_median as f64 * 1000.0 / wall_median as f64 * 100.0;
    let harness_pct = 100.0 - svm_pct;
    println!("SVM (solana-program-runtime + solana-sbpf): {:>5.1}%  ({:.1} µs)",
        svm_pct, svm_median as f64);
    println!("Harness (mollusk overhead):                 {:>5.1}%  ({:.1} µs)",
        harness_pct, harness_median as f64 / 1000.0);
    println!();

    // Per-account data sizes
    println!("--- Account Data Sizes ---");
    println!();
    let mut account_sizes: Vec<_> = accounts.iter()
        .map(|(k, a)| (k, a.data.len(), a.owner))
        .collect();
    account_sizes.sort_by(|a, b| b.1.cmp(&a.1));
    for (key, size, owner) in account_sizes.iter().take(10) {
        let key_str = key.to_string();
        let short_key = &key_str[..8];
        let owner_str = owner.to_string();
        let short_owner = &owner_str[..8];
        println!("  {short_key}.. ({short_owner}..): {:>6} bytes", size);
    }
}
