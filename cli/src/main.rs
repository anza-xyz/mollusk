//! Mollusk CLI.

mod config;
mod runner;

use {
    crate::runner::{ProtoLayout, Runner},
    clap::{Parser, Subcommand},
    config::ConfigFile,
    mollusk_svm::{result::Compare, Mollusk},
    solana_pubkey::Pubkey,
    std::{fs, path::Path, str::FromStr},
};

#[derive(Subcommand)]
enum SubCommand {
    /// Execute a fixture using Mollusk and inspect the effects.
    ExecuteFixture {
        /// The path to the ELF file.
        #[arg(required = true)]
        elf_path: String,
        /// Path to an instruction fixture (`.fix` file) or a directory
        /// containing them.
        #[arg(required = true)]
        fixture: String,
        /// The ID to use for the program.
        #[arg(value_parser = Pubkey::from_str)]
        program_id: Pubkey,

        /// Path to the config file for validation checks.
        #[arg(short, long)]
        config: Option<String>,
        /// Just execute the fixture without any validation.
        #[arg(short, long)]
        inputs_only: bool,
        /// Enable emission of program logs to stdout. Disabled by default.
        #[arg(long)]
        program_logs: bool,
        /// Protobuf layout to use when executing the fixture.
        #[arg(long, default_value = "mollusk")]
        proto: ProtoLayout,
        /// Enable verbose mode for fixture effects. Does not enable program
        /// logs. Disabled by default.
        #[arg(short, long)]
        verbose: bool,
    },
    /// Execute a fixture across two Mollusk instances to compare the results
    /// of two versions of a program.
    RunTest {
        /// The path to the ELF file of the "ground truth" program.
        #[arg(required = true)]
        elf_path_source: String,
        /// The path to the ELF file of the test program. This is the program
        /// that will be tested against the ground truth.
        #[arg(required = true)]
        elf_path_target: String,
        /// Path to an instruction fixture (`.fix` file) or a directory
        /// containing them.
        #[arg(required = true)]
        fixture: String,
        /// The ID to use for the program.
        #[arg(value_parser = Pubkey::from_str)]
        program_id: Pubkey,

        /// Path to the config file for validation checks.
        #[arg(short, long)]
        config: Option<String>,
        /// Enable emission of program logs to stdout. Disabled by default.
        #[arg(long)]
        program_logs: bool,
        /// Protobuf layout to use when executing the fixture.
        #[arg(long, default_value = "mollusk")]
        proto: ProtoLayout,
        /// Enable verbose mode for fixture effects. Does not enable program
        /// logs. Disabled by default.
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    pub command: SubCommand,
}

fn search_paths(path: &str, extension: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    fn search_path_recursive(
        path: &Path,
        extension: &str,
        result: &mut Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                search_path_recursive(&entry?.path(), extension, result)?;
            }
        } else if path.extension().is_some_and(|ext| ext == extension) {
            result.push(path.to_str().unwrap().to_string());
        }
        Ok(())
    }

    let mut result = Vec::new();
    search_path_recursive(Path::new(path), extension, &mut result)?;
    Ok(result)
}

fn add_elf_to_mollusk(mollusk: &mut Mollusk, elf_path: &str, program_id: &Pubkey) {
    let elf = mollusk_svm::file::read_file(elf_path);
    mollusk.add_program_with_elf_and_loader(
        program_id,
        &elf,
        &solana_sdk_ids::bpf_loader_upgradeable::id(),
    );
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        SubCommand::ExecuteFixture {
            elf_path,
            fixture,
            program_id,
            config,
            inputs_only,
            program_logs,
            proto,
            verbose,
        } => {
            let mut mollusk = Mollusk::default();
            add_elf_to_mollusk(&mut mollusk, &elf_path, &program_id);

            let checks = if let Some(config_path) = config {
                ConfigFile::try_load(&config_path)?.checks
            } else {
                // Defaults to all checks.
                Compare::everything()
            };

            let runner = Runner::new(checks, inputs_only, program_logs, proto, verbose);

            for fixture_path in search_paths(&fixture, "fix")? {
                runner.run(&mut mollusk, None, &fixture_path)?;
            }
        }
        SubCommand::RunTest {
            elf_path_source,
            elf_path_target,
            fixture,
            program_id,
            config,
            program_logs,
            proto,
            verbose,
        } => {
            // First, set up a Mollusk instance with the ground truth program.
            let mut mollusk_ground = Mollusk::default();
            add_elf_to_mollusk(&mut mollusk_ground, &elf_path_source, &program_id);

            // Next, set up a Mollusk instance with the test program.
            let mut mollusk_test = Mollusk::default();
            add_elf_to_mollusk(&mut mollusk_test, &elf_path_target, &program_id);

            let checks = if let Some(config_path) = config {
                ConfigFile::try_load(&config_path)?.checks
            } else {
                // Defaults to all checks.
                Compare::everything()
            };

            let runner = Runner::new(
                checks,
                /* inputs_only */ true,
                program_logs,
                proto,
                verbose,
            );

            let mut failures = 0;
            for fixture_path in search_paths(&fixture, "fix")? {
                let result =
                    runner.run(&mut mollusk_ground, Some(&mut mollusk_test), &fixture_path)?;
                if !result {
                    failures += 1;
                }
            }

            println!();
            println!("[DONE][TEST RESULT]: {} failures", failures);
        }
    }
    Ok(())
}
