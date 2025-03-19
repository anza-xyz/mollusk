//! CLI runner. Many jobs share the same pattern but do different core actions.

use {
    clap::ValueEnum,
    mollusk_svm::{
        result::{Compare, Config, InstructionResult},
        Mollusk,
    },
    mollusk_svm_bencher::{get_solana_version, result::MolluskComputeUnitBenchResult},
    std::path::PathBuf,
};

#[derive(Clone, Debug, Default, ValueEnum)]
pub enum ProtoLayout {
    /// Use Mollusk protobuf layouts.
    #[default]
    Mollusk,
    /// Use Firedancer protobuf layouts.
    Firedancer,
}

pub struct Runner {
    checks: Vec<Compare>,
    cus_report: Option<String>,
    inputs_only: bool,
    program_logs: bool,
    proto: ProtoLayout,
    verbose: bool,
}

impl Runner {
    pub fn new(
        checks: Vec<Compare>,
        cus_report: Option<String>,
        inputs_only: bool,
        program_logs: bool,
        proto: ProtoLayout,
        verbose: bool,
    ) -> Self {
        Self {
            checks,
            cus_report,
            inputs_only,
            program_logs,
            proto,
            verbose,
        }
    }

    // Returns the result from the instruction, and the effects converted to
    // `InstrucionResult`.
    fn run_fixture(
        &self,
        mollusk: &mut Mollusk,
        fixture_path: &str,
    ) -> (InstructionResult, InstructionResult) {
        match self.proto {
            ProtoLayout::Mollusk => {
                let fixture = mollusk_svm_fuzz_fixture::Fixture::load_from_blob_file(fixture_path);
                let result = mollusk.process_fixture(&fixture);
                let effects = (&fixture.output).into();
                (result, effects)
            }
            ProtoLayout::Firedancer => {
                let fixture =
                    mollusk_svm_fuzz_fixture_firedancer::Fixture::load_from_blob_file(fixture_path);
                let result = mollusk.process_firedancer_fixture(&fixture);
                let (_, effects) = mollusk_svm::fuzz::firedancer::load_firedancer_fixture(&fixture);
                (result, effects)
            }
        }
    }

    pub fn run(
        &self,
        ground: Option<&mut Mollusk>,
        target: &mut Mollusk,
        fixture_path: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Disable stdout logging of program logs if not specified.
        if !self.program_logs {
            solana_logger::setup_with("");
        }

        let mut pass = true;
        let mut bench_results = Vec::new();

        if self.verbose {
            println!("----------------------------------------");
        }

        let ground_result = ground.map(|ground| {
            // Command `run-test`.

            if self.verbose {
                println!("[GROUND]: FIX: {}", fixture_path);
            }

            if self.program_logs {
                println!("[GROUND]: Program logs:");
                println!();
            }

            let (ground_result, effects) = self.run_fixture(ground, fixture_path);

            if self.program_logs {
                println!();
            }

            if self.verbose {
                println!("[GROUND]: Result:");
                println!();
                println!("{:?}", &ground_result);
                println!();
            }

            if !self.inputs_only {
                // Compare against the effects.
                if self.verbose {
                    println!("[GROUND]: Comparing against fixture effects...");
                    println!();
                }

                pass &= ground_result.compare_with_config(
                    &effects,
                    &self.checks,
                    &Config {
                        panic: false,
                        verbose: self.verbose,
                    },
                );
            }

            ground_result
        });

        // All commands have a target.

        if self.verbose {
            println!("[TARGET]: FIX: {}", &fixture_path);
        }

        if self.program_logs {
            println!("[TARGET]: Program logs:");
            println!();
        }

        let (target_result, effects) = self.run_fixture(target, fixture_path);

        if self.cus_report.is_some() {
            let fixture_name = parse_fixture_name(fixture_path);
            let bench_result =
                MolluskComputeUnitBenchResult::new(fixture_name, target_result.clone());
            bench_results.push(bench_result);
        }

        if self.program_logs {
            println!();
        }

        if self.verbose {
            println!("[TARGET]: Result:");
            println!();
            println!("{:?}", &target_result);
            println!();
        }

        if !self.inputs_only {
            // Compare against the effects.
            if self.verbose {
                println!("[TARGET]: Comparing against fixture effects...");
                println!();
            }

            pass &= target_result.compare_with_config(
                &effects,
                &self.checks,
                &Config {
                    panic: false,
                    verbose: self.verbose,
                },
            );
        }

        if let Some(ground_result) = ground_result {
            // Compare the two results.
            if self.verbose {
                println!("[TEST]: Comparing the two results...");
                println!();
            }

            pass &= ground_result.compare_with_config(
                &target_result,
                &self.checks,
                &Config {
                    panic: false,
                    verbose: self.verbose,
                },
            );
        }

        if self.verbose {
            println!();
        }

        if pass {
            println!("PASS: {}", &fixture_path);
        } else {
            println!("FAIL: {}", &fixture_path);
        }

        if self.verbose {
            println!("----------------------------------------");
            println!();
        }

        if let Some(cus_report) = &self.cus_report {
            let solana_version = get_solana_version();
            mollusk_svm_bencher::result::write_results(
                &PathBuf::from(cus_report),
                &solana_version,
                bench_results,
            );
        }

        Ok(pass)
    }

    pub fn run_all(
        &self,
        mut ground: Option<&mut Mollusk>,
        target: &mut Mollusk,
        fixtures: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut failures = 0;

        for fixture_path in fixtures {
            let result = self.run(ground.as_deref_mut(), target, fixture_path)?;

            if !result {
                failures += 1;
            }
        }

        println!();
        println!("[DONE][TEST RESULT]: {} failures", failures);

        if failures > 0 {
            std::process::exit(1);
        }

        Ok(())
    }
}

fn parse_fixture_name(fixture_path: &str) -> &str {
    fixture_path
        .rsplit_once('/')
        .map_or(fixture_path, |(_, name)| name)
        .split_once('.')
        .map_or_else(|| fixture_path, |(name, _)| name)
}
