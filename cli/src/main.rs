//! Mollusk CLI.

mod config;
mod matrix;
mod runner;

use {
    crate::runner::{ProtoLayout, Runner},
    bs58,
    clap::{Parser, Subcommand},
    config::ConfigFile,
    mollusk_svm::{result::Compare, Mollusk},
    runner::CusReport,
    solana_pubkey::Pubkey,
    std::{collections::HashMap, fs, path::Path, str::FromStr},
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
        /// Protobuf layout to use when executing the fixture.
        #[arg(long, default_value = "mollusk")]
        proto: ProtoLayout,
        /// Enable verbose mode for fixture effects. Does not enable program
        /// logs. Disabled by default.
        #[arg(short, long)]
        verbose: bool,
    },
    /// Run fixtures under a baseline FeatureSet and automatically generated
    /// feature variants, then compare parity and compute units.
    Matrix {
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

        /// Repeatable feature ID to include in the matrix (base58 pubkey).
        #[arg(long)]
        feature: Vec<String>,
        /// Maximum absolute compute unit delta threshold for pass/fail.
        #[arg(long)]
        max_cu_delta_abs: Option<u64>,
        /// Maximum percentage compute unit delta threshold for pass/fail.
        #[arg(long)]
        max_cu_delta_percent: Option<f32>,

        /// Output directory for generated reports.
        #[arg(long)]
        out_dir: Option<String>,
        /// Repeatable output format: one of "markdown" or "json". Defaults to
        /// ["markdown"] if not provided.
        #[arg(long, value_parser = ["markdown", "json"])]
        output: Vec<String>,

        #[arg(long, default_value = "mollusk")]
        proto: ProtoLayout,
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

/// A variant that can be applied and reset on a Mollusk instance.
struct Variant {
    name: String,
    apply: Box<dyn Fn(&mut Mollusk)>,
    reset: Box<dyn Fn(&mut Mollusk)>,
}

/// Generate feature variants from a list of feature pubkeys.
fn generate_feature_variants(feature_ids: &[solana_pubkey::Pubkey]) -> Vec<Variant> {
    if feature_ids.is_empty() {
        return Vec::new();
    }

    let n = feature_ids.len();
    let total = 1usize << n;
    let mut variants = Vec::with_capacity(total - 1);

    for mask in 1..total {
        let mut enabled_features = Vec::new();
        let mut parts: Vec<String> = Vec::new();

        for (idx, &fid) in feature_ids.iter().enumerate().take(n) {
            if (mask >> idx) & 1 == 1 {
                enabled_features.push(fid);
                let short = bs58::encode(fid.to_bytes()).into_string();
                parts.push(short.chars().take(8).collect());
            }
        }

        let name = parts.join("+");
        let enabled_copy = enabled_features.clone();
        let disabled_copy = enabled_features.clone();

        variants.push(Variant {
            name,
            apply: Box::new(move |mollusk: &mut Mollusk| {
                for &feature_id in &enabled_copy {
                    mollusk.feature_set.active_mut().insert(feature_id, 0);
                }
            }),
            reset: Box::new(move |mollusk: &mut Mollusk| {
                for &feature_id in &disabled_copy {
                    mollusk.feature_set.active_mut().remove(&feature_id);
                }
            }),
        });
    }

    variants
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
        SubCommand::Matrix {
            elf_path,
            fixture,
            program_id,
            feature,
            max_cu_delta_abs,
            max_cu_delta_percent,
            out_dir,
            output,
            proto,
        } => {
            let mut mollusk = Mollusk::default();
            add_elf_to_mollusk(&mut mollusk, &elf_path, &program_id);

            let fm = matrix::FeatureMatrix::new(mollusk)
            .thresholds(matrix::Thresholds {
                max_cu_delta_abs,
                max_cu_delta_percent,
            });

            // Collect and validate feature pubkeys upfront
            let feature_ids: Vec<solana_pubkey::Pubkey> = if feature.is_empty() {
                Vec::new()
            } else {
                let ids: Vec<solana_pubkey::Pubkey> = feature
                    .iter()
                    .filter_map(|s| bs58::decode(s).into_vec().ok())
                    .filter(|v| v.len() == 32)
                    .map(|v| solana_pubkey::Pubkey::new_from_array(v.try_into().unwrap()))
                    .collect();
                if ids.is_empty() {
                    return Err("no valid --feature ids provided".into());
                }
                // guardrail: hard cap variants to avoid explosion
                const MAX_VARIANTS: usize = 1 << 12; // 4096
                let n = ids.len();
                let total = 1usize << n;
                if total > MAX_VARIANTS {
                    return Err(format!(
                        "requested {} features generates {} variants; cap is {}",
                        n,
                        total - 1,
                        MAX_VARIANTS - 1
                    )
                    .into());
                }
                ids
            };

            let variants = generate_feature_variants(&feature_ids);

            let fixtures = search_paths(&fixture, "fix")?;
            if matches!(proto, ProtoLayout::Firedancer) {
                return Err(
                    "Firedancer fixtures are not supported by the matrix runner yet. Use --proto \
                     mollusk."
                        .into(),
                );
            }

            let mut aggregates: HashMap<String, mollusk_svm::result::InstructionResult> =
                HashMap::new();
            aggregates.entry("baseline".to_string()).or_default();
            for v in &variants {
                aggregates.entry(v.name.clone()).or_default();
            }

            for path in &fixtures {
                let fixture = mollusk_svm_fuzz_fixture::Fixture::load_from_blob_file(path);

                // Create a single mollusk instance with the fixture's feature set
                let mut mollusk = fm.build_mollusk_for_featureset(&fixture.input.feature_set);
                add_elf_to_mollusk(&mut mollusk, &elf_path, &program_id);

                // Run baseline
                let baseline_result = mollusk.process_fixture(&fixture);
                let entry = aggregates.entry("baseline".to_string()).or_default();
                entry.absorb(baseline_result);

                // Run variants
                for variant in &variants {
                    (variant.apply)(&mut mollusk);
                    let variant_result = mollusk.process_fixture(&fixture);
                    let entry = aggregates.entry(variant.name.clone()).or_default();
                    entry.absorb(variant_result);
                    (variant.reset)(&mut mollusk);
                }
            }

            let mut runs: Vec<matrix::VariantRun<mollusk_svm::result::InstructionResult>> =
                Vec::new();
            runs.push(matrix::VariantRun {
                name: "baseline".to_string(),
                output: aggregates.remove("baseline").unwrap_or_default(),
            });
            for variant in variants {
                runs.push(matrix::VariantRun {
                    name: variant.name.clone(),
                    output: aggregates.remove(&variant.name).unwrap_or_default(),
                });
            }

            // Derive report formats. Default to markdown when no --output given.
            let emit_markdown = output.is_empty() || output.iter().any(|s| s == "markdown");
            let emit_json = output.iter().any(|s| s == "json");

            let fm = fm
                .report(matrix::ReportConfig {
                    out_dir: out_dir.clone().map(Into::into),
                    markdown: emit_markdown,
                    json: emit_json,
                })
                .build();

            let (_md, _js) = fm.generate_reports(&runs)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_parsing_with_features() {
        let args = vec![
            "mollusk",
            "matrix",
            "./programs/token/src/elf/token.so",
            "./harness/tests",
            "11111111111111111111111111111112",
            "--feature",
            "2yDdYzg56LgRBzX1UeMNcrsXphJ4yTZe4cCvEoGzbXDc",
            "--feature",
            "5GDFSoKFJWWEkttfwTwWXfehKH7DGiL9v8bNChjJ5Q5g",
            "--output",
            "markdown",
        ];

        let parsed = Cli::try_parse_from(args);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_matrix_rejects_baseline_slot_flag() {
        let args = vec![
            "mollusk",
            "matrix",
            "./programs/token/src/elf/token.so",
            "./harness/tests",
            "11111111111111111111111111111112",
            "--baseline-slot",
            "250000000",
        ];
        let parsed = Cli::try_parse_from(args);
        assert!(parsed.is_err());
    }
}
