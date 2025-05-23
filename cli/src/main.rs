//! Mollusk CLI.

mod config;
mod runner;

use {
    crate::runner::{ProtoLayout, Runner},
    clap::{Parser, Subcommand},
    config::ConfigFile,
    mollusk_svm::{result::Compare, Mollusk},
    runner::CusReport,
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
        /// Directory to write a compute unit consumption report.
        #[arg(long)]
        cus_report: Option<String>,
        /// Table header for the compute unit consumption report.
        ///
        /// Note this flag is ignored if `cus_report` is not set.
        #[arg(long)]
        cus_report_table_header: Option<String>,
        /// Skip comparing compute unit consumption, but compare everything
        /// else.
        ///
        /// Note this flag is ignored if `inputs_only` is set, and will
        /// override a `Compare::ComputeUnits` check in the config file.
        #[arg(long)]
        ignore_compute_units: bool,
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
        /// Directory to write a compute unit consumption report.
        #[arg(long)]
        cus_report: Option<String>,
        /// Table header for the compute unit consumption report.
        ///
        /// Note this flag is ignored if `cus_report` is not set.
        #[arg(long)]
        cus_report_table_header: Option<String>,
        /// Skip comparing compute unit consumption, but compare everything
        /// else.
        ///
        /// Note this flag will override a `Compare::ComputeUnits` check in the
        /// config file.
        #[arg(long)]
        ignore_compute_units: bool,
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
            cus_report,
            cus_report_table_header,
            ignore_compute_units,
            inputs_only,
            program_logs,
            proto,
            verbose,
        } => {
            let mut mollusk = Mollusk::default();
            add_elf_to_mollusk(&mut mollusk, &elf_path, &program_id);

            let checks = if let Some(config_path) = config {
                ConfigFile::try_load(&config_path)?.checks
            } else if ignore_compute_units {
                Compare::everything_but_cus()
            } else {
                // Defaults to all checks.
                Compare::everything()
            };

            let fixtures = search_paths(&fixture, "fix")?;

            Runner::new(
                checks,
                cus_report.map(|path| CusReport::new(path, cus_report_table_header)),
                inputs_only,
                program_logs,
                proto,
                verbose,
            )
            .run_all(None, &mut mollusk, &fixtures)?
        }
        SubCommand::RunTest {
            elf_path_source,
            elf_path_target,
            fixture,
            program_id,
            config,
            cus_report,
            cus_report_table_header,
            ignore_compute_units,
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
            } else if ignore_compute_units {
                Compare::everything_but_cus()
            } else {
                // Defaults to all checks.
                Compare::everything()
            };

            let fixtures = search_paths(&fixture, "fix")?;

            Runner::new(
                checks,
                cus_report.map(|path| CusReport::new(path, cus_report_table_header)),
                /* inputs_only */ true,
                program_logs,
                proto,
                verbose,
            )
            .run_all(Some(&mut mollusk_ground), &mut mollusk_test, &fixtures)?
        }
    }
    Ok(())
}
