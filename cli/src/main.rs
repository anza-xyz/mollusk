//! Mollusk CLI.

mod config;
mod runner;

#[cfg(feature = "feature-matrix")]
use clap::ValueEnum;
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
    #[cfg(feature = "feature-matrix")]
    Matrix {
        #[arg(required = true)]
        elf_path: String,
        #[arg(required = true)]
        fixtures_dir: String,
        #[arg(value_parser = Pubkey::from_str)]
        program_id: Pubkey,
        #[arg(long)]
        variant: Vec<String>,
        #[arg(long)]
        feature: Vec<String>,
        #[arg(long, default_value = "cartesian")]
        matrix: MatrixStrategy,
        #[arg(long)]
        max_cu_delta_abs: Option<u64>,
        #[arg(long)]
        max_cu_delta_percent: Option<f32>,
        #[arg(long)]
        out_dir: Option<String>,
        #[arg(long)]
        markdown: bool,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "mollusk")]
        proto: ProtoLayout,
    },
}

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    pub command: SubCommand,
}

#[cfg(feature = "feature-matrix")]
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum MatrixStrategy {
    Cartesian,
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
        #[cfg(feature = "feature-matrix")]
        SubCommand::Matrix {
            elf_path,
            fixtures_dir,
            program_id,
            variant,
            feature,
            matrix,
            max_cu_delta_abs,
            max_cu_delta_percent,
            out_dir,
            markdown,
            json,
            proto,
        } => {
            use bs58;
            use mollusk_svm::feature_matrix::{
                BaselineConfig, FeatureMatrix, FeatureVariant, ReportConfig, Thresholds,
            };
            let mut mollusk = Mollusk::default();
            add_elf_to_mollusk(&mut mollusk, &elf_path, &program_id);

            let fm = FeatureMatrix::new(
                mollusk,
                BaselineConfig::Explicit(mollusk_svm::Mollusk::default().feature_set.clone()),
            )
            .thresholds(Thresholds {
                max_cu_delta_abs,
                max_cu_delta_percent,
                require_result_parity: true,
            });

            let mut variants_list: Vec<FeatureVariant> = Vec::new();
            for spec in &variant {
                let (name, list) = match spec.split_once(':') {
                    Some(v) => v,
                    None => {
                        eprintln!(
                            "[matrix] ignoring malformed --variant (missing ':'): {}",
                            spec
                        );
                        continue;
                    }
                };
                let ids: Vec<solana_pubkey::Pubkey> = list
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| bs58::decode(s).into_vec().ok())
                    .filter(|v| v.len() == 32)
                    .map(|v| solana_pubkey::Pubkey::new_from_array(v.try_into().unwrap()))
                    .collect();
                if ids.is_empty() {
                    eprintln!(
                        "[matrix] ignoring --variant '{}' with no valid feature ids",
                        name
                    );
                    continue;
                }
                variants_list.push(FeatureVariant {
                    name: name.to_string(),
                    enable: ids,
                });
            }
            if !feature.is_empty() {
                let feature_ids: Vec<solana_pubkey::Pubkey> = feature
                    .iter()
                    .filter_map(|s| bs58::decode(s).into_vec().ok())
                    .filter(|v| v.len() == 32)
                    .map(|v| solana_pubkey::Pubkey::new_from_array(v.try_into().unwrap()))
                    .collect();
                if feature_ids.is_empty() {
                    eprintln!("[matrix] no valid --feature ids provided");
                } else {
                    match matrix {
                        MatrixStrategy::Cartesian => {
                            let n = feature_ids.len();
                            let total = 1usize << n;
                            for mask in 1..total {
                                let mut ids = Vec::new();
                                let mut parts: Vec<String> = Vec::new();
                                for (idx, fid) in feature_ids.iter().enumerate().take(n) {
                                    if (mask >> idx) & 1 == 1 {
                                        let id = *fid;
                                        ids.push(id);
                                        let short = bs58::encode(id.to_bytes()).into_string();
                                        parts.push(short.chars().take(8).collect());
                                    }
                                }
                                let name = parts.join("+");
                                variants_list.push(FeatureVariant { name, enable: ids });
                            }
                        }
                    }
                }
            }

            let fixtures = search_paths(&fixtures_dir, "fix")?;
            if matches!(proto, ProtoLayout::Firedancer) {
                eprintln!("[matrix] Firedancer fixtures are not supported by the matrix runner yet. Use --proto mollusk.");
                std::process::exit(2);
            }

            use std::collections::HashMap;
            let mut aggregates: HashMap<String, mollusk_svm::result::InstructionResult> =
                HashMap::new();
            aggregates.entry("baseline".to_string()).or_default();
            for v in &variants_list {
                aggregates.entry(v.name.clone()).or_default();
            }

            for path in &fixtures {
                let fixture = mollusk_svm_fuzz_fixture::Fixture::load_from_blob_file(path);
                let base_fs = fixture.input.feature_set.clone();

                {
                    let features = base_fs.clone();
                    let mut m = fm.build_mollusk_for_featureset(&features);
                    add_elf_to_mollusk(&mut m, &elf_path, &program_id);
                    let res = m.process_fixture(&fixture);
                    let entry = aggregates.entry("baseline".to_string()).or_default();
                    let mut total = entry.clone();
                    total.absorb(res);
                    *entry = total;
                }

                for v in &variants_list {
                    let features = fm.apply_variant(&base_fs, v);
                    let mut m = fm.build_mollusk_for_featureset(&features);
                    add_elf_to_mollusk(&mut m, &elf_path, &program_id);
                    let res = m.process_fixture(&fixture);
                    let entry = aggregates.entry(v.name.clone()).or_default();
                    let mut total = entry.clone();
                    total.absorb(res);
                    *entry = total;
                }
            }

            let mut runs: Vec<
                mollusk_svm::feature_matrix::VariantRun<mollusk_svm::result::InstructionResult>,
            > = Vec::new();
            let meta_base = fm.resolve_baseline_featureset();
            runs.push(mollusk_svm::feature_matrix::VariantRun {
                name: "baseline".to_string(),
                features: meta_base.clone(),
                output: aggregates.remove("baseline").unwrap_or_default(),
            });
            for v in variants_list {
                runs.push(mollusk_svm::feature_matrix::VariantRun {
                    name: v.name.clone(),
                    features: meta_base.clone(),
                    output: aggregates.remove(&v.name).unwrap_or_default(),
                });
            }

            let fm = fm
                .report(ReportConfig {
                    out_dir: out_dir.clone().map(Into::into),
                    markdown,
                    json,
                })
                .build();

            let (_md, _js) = fm.generate_reports(&runs);
        }
        #[cfg(not(feature = "feature-matrix"))]
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "feature-matrix")]
    fn test_matrix_parsing_with_features_and_strategy() {
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
            "--matrix",
            "cartesian",
            "--markdown",
        ];

        let parsed = Cli::try_parse_from(args);
        assert!(parsed.is_ok());
    }

    #[test]
    #[cfg(feature = "feature-matrix")]
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
