//! The Mollusk Compute Unit Bencher can be used to benchmark the compute unit
//! usage of Solana programs. It provides a simple API for developers to write
//! benchmarks for their programs, which can be checked while making changes to
//! the program.
//!
//! A markdown file is generated, which captures all of the compute unit
//! benchmarks. If a benchmark has a previous value, the delta is also
//! recorded. This can be useful for developers to check the implications of
//! changes to the program on compute unit usage.
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

mod result;

use {
    mollusk_svm::{result::ProgramResult, Mollusk},
    result::{write_results, MolluskComputeUnitBenchResult},
    solana_account::Account,
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
    std::path::PathBuf,
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
        write_results(&self.out_dir, bench_results);
    }
}
