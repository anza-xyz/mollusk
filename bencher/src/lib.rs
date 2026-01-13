//! The Mollusk Compute Unit Bencher can be used to benchmark the compute unit
//! usage of Solana programs. It provides a simple API for developers to write
//! benchmarks for their programs, or compare multiple implementations of their
//! programs in a matrix, which can be checked while making changes to the
//! program.
//!
//! A markdown file is generated, which captures all of the compute unit
//! benchmarks. In the case of single program if a benchmark has a previous
//! value, the delta is also recorded. This can be useful for developers to
//! check the implications of changes to the program on compute unit usage.
//!
//! ```rust,ignore
//! use {
//!     mollusk_svm_bencher::MolluskComputeUnitBencher,
//!     mollusk_svm::Mollusk,
//!     /* ... */
//! };
//!
//! // Optionally disable logging.
//! solana_logger::setup_with("");
//!
//! /* Instruction & accounts setup ... */
//!
//! let mollusk = Mollusk::new(&program_id, "my_program");
//!
//! MolluskComputeUnitBencher::new(mollusk)
//!     .bench(("bench0", &instruction0, &accounts0))
//!     .bench(("bench1", &instruction1, &accounts1))
//!     .bench(("bench2", &instruction2, &accounts2))
//!     .bench(("bench3", &instruction3, &accounts3))
//!     .must_pass(true)
//!     .out_dir("../target/benches")
//!     .execute();
//! ```
//!
//! The `must_pass` argument can be provided to trigger a panic if any defined
//! benchmark tests do not pass. `out_dir` specifies the directory where the
//! markdown file will be written.
//!
//! Developers can invoke this benchmark test with `cargo bench`. They may need
//! to add a bench to the project's `Cargo.toml`.
//!
//! ```toml
//! [[bench]]
//! name = "compute_units"
//! harness = false
//! ```
//!
//! The markdown file will contain entries according to the defined benchmarks.
//!
//! ```markdown
//! | Name   | CUs   | Delta  |
//! |--------|-------|--------|
//! | bench0 | 450   | --     |
//! | bench1 | 579   | -129   |
//! | bench2 | 1,204 | +754   |
//! | bench3 | 2,811 | +2,361 |
//! ```
//! ### Matrix Benchmarking
//!
//! If you want to compare multiple program implementations (e.g., comparing
//! an optimized version against a baseline), use
//! `MolluskComputeUnitMatrixBencher`. This generates a table where each program
//! is a column.
//!
//! ```rust,ignore
//! use {
//!     mollusk_svm_bencher::MolluskComputeUnitMatrixBencher,
//!     mollusk_svm::Mollusk,
//!     /* ... */
//! };
//!
//! /* Instruction & accounts setup ... */
//!
//! let mollusk = Mollusk::new(&program_id, "program_v1");
//!
//! MolluskComputeUnitMatrixBencher::new(mollusk)
//!     .programs(&["program_v1", "program_v2", "program_v3"])
//!     .bench(("bench0", &instruction0, &accounts0))
//!     .bench(("bench1", &instruction1, &accounts1))
//!     .must_pass(true)
//!     .out_dir("../target/benches")
//!     .execute();
//! ```
//! The matrix markdown file will contain entries comparing all provided
//! programs.
//!
//! ```markdown
//! | Name     | CU (`program_v1`) | CU (`program_v2`) | CU (`program_v3`) |
//! |----------|-------------------|-------------------|-------------------|
//! | `bench0` | 1,400             | 1,390             | 1,385             |
//! | `bench1` | 2,100             | 2,050             | 2,045             |
//! ```

pub mod result;

use {
    chrono::Utc,
    mollusk_svm::{result::ProgramResult, Mollusk},
    result::{
        mx_write_results, write_results, MolluskComputeUnitBenchResult,
        MolluskComputeUnitMatrixBenchResult,
    },
    solana_account::Account,
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
    std::{path::PathBuf, process::Command},
};

/// A bench is a tuple of a name, an instruction, and a list of accounts.
pub type Bench<'a> = (&'a str, &'a Instruction, &'a [(Pubkey, Account)]);

/// Mollusk's compute unit bencher.
///
/// Allows developers to bench test compute unit usage on their programs.
pub struct MolluskComputeUnitBencher<'a> {
    benches: Vec<Bench<'a>>,
    mollusk: Mollusk,
    must_pass: bool,
    out_dir: PathBuf,
}

impl<'a> MolluskComputeUnitBencher<'a> {
    /// Create a new bencher, to which benches and configurations can be added.
    pub fn new(mollusk: Mollusk) -> Self {
        let mut out_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        out_dir.push("benches");
        Self {
            benches: Vec::new(),
            mollusk,
            must_pass: false,
            out_dir,
        }
    }

    /// Add a bench to the bencher.
    pub fn bench(mut self, bench: Bench<'a>) -> Self {
        self.benches.push(bench);
        self
    }

    /// Set whether the bencher should panic if a program execution fails.
    pub const fn must_pass(mut self, must_pass: bool) -> Self {
        self.must_pass = must_pass;
        self
    }

    /// Set the output directory for the results.
    pub fn out_dir(mut self, out_dir: &str) -> Self {
        self.out_dir = PathBuf::from(out_dir);
        self
    }

    /// Execute the benches.
    pub fn execute(&mut self) {
        let table_header = Utc::now().to_string();
        let solana_version = get_solana_version();
        let bench_results = std::mem::take(&mut self.benches)
            .into_iter()
            .map(|(name, instruction, accounts)| {
                let result = self.mollusk.process_instruction(instruction, accounts);
                match result.program_result {
                    ProgramResult::Success => (),
                    _ => {
                        if self.must_pass {
                            panic!(
                                "Program execution failed, but `must_pass` was set. Error: {:?}",
                                result.program_result
                            );
                        }
                    }
                }
                MolluskComputeUnitBenchResult::new(name, result)
            })
            .collect::<Vec<_>>();
        write_results(&self.out_dir, &table_header, &solana_version, bench_results);
    }
}

/// Mollusk's matrix compute unit bencher.
///
/// Allows developers to bench test compute unit usage on multiple
/// implementations of their programs.
pub struct MolluskComputeUnitMatrixBencher<'a> {
    mollusk: &'a mut Mollusk,
    program_names: Vec<&'a str>,
    benches: Vec<Bench<'a>>,
    must_pass: bool,
    out_dir: PathBuf,
}

impl<'a> MolluskComputeUnitMatrixBencher<'a> {
    /// Create a new matrix bencher, to which benches and configurations can be
    /// added.
    pub fn new(mollusk: &'a mut Mollusk) -> Self {
        let mut out_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        out_dir.push("benches");
        Self {
            mollusk,
            program_names: Vec::new(),
            benches: Vec::new(),
            must_pass: false,
            out_dir,
        }
    }

    /// Add the program names to be benched.
    pub fn programs(mut self, names: &[&'a str]) -> Self {
        self.program_names = names.to_vec();
        self
    }

    /// Add a bench to the bencher.
    pub fn bench(mut self, bench: Bench<'a>) -> Self {
        self.benches.push(bench);
        self
    }

    /// Set whether the bencher should panic if a program execution fails.
    pub fn must_pass(mut self, must_pass: bool) -> Self {
        self.must_pass = must_pass;
        self
    }

    /// Set the output directory for the results.
    pub fn out_dir(mut self, out_dir: &str) -> Self {
        self.out_dir = PathBuf::from(out_dir);
        self
    }

    /// Execute the benches.
    pub fn execute(&mut self) {
        let table_header = Utc::now().to_string();
        let solana_version = get_solana_version();

        let mut bench_results: Vec<MolluskComputeUnitMatrixBenchResult> = Vec::new();
        for program_name in &self.program_names {
            // Extract the program ID from the first instruction.
            if let Some((_, first_instruction, _)) = self.benches.first() {
                self.mollusk
                    .add_program(&first_instruction.program_id, program_name);
            }

            let mut ix_results = MolluskComputeUnitMatrixBenchResult::new(program_name);

            for (ix_name, instruction, accounts) in &self.benches {
                let result = self.mollusk.process_instruction(instruction, accounts);
                match result.program_result {
                    ProgramResult::Success => (),
                    _ => {
                        if self.must_pass {
                            panic!(
                                "Program execution failed, but `must_pass` was set. Error: {:?}",
                                result.program_result
                            );
                        }
                    }
                }
                ix_results.add_result(ix_name, result);
            }
            bench_results.push(ix_results);
        }

        mx_write_results(
            &self.out_dir,
            &table_header,
            &solana_version,
            &bench_results,
        );
    }
}

pub fn get_solana_version() -> String {
    match Command::new("solana").arg("--version").output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "Unknown".to_string(),
    }
}
