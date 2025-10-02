//! Feature-activation matrix runner for testing program behavior across different FeatureSet configurations.
//!
//! This module provides functionality to run fixtures under different feature combinations
//! and compare results. Currently only used by the CLI, but kept in the harness for
//! potential reuse by other tools.

use {
    crate::{program::ProgramCache, Mollusk},
    agave_feature_set::FeatureSet,
    mollusk_svm_result::{self as result_model, Config as ResultConfig},
    solana_pubkey::Pubkey,
};

/// Baseline selector for feature-activation comparisons.
#[derive(Clone, Debug)]
pub enum BaselineConfig {
    /// Use the `FeatureSet` active at a specific slot.
    Slot(u64),
    /// Use an explicit `FeatureSet`.
    Explicit(FeatureSet),
}

/// A named variant of features to enable for comparison.
#[derive(Clone, Debug, Default)]
pub struct FeatureVariant {
    /// Human-friendly name for reports.
    pub name: String,
    /// Features to explicitly enable on top of the baseline.
    pub enable: Vec<Pubkey>,
}

/// Thresholds used to flag regressions in comparisons.
#[derive(Clone, Debug)]
pub struct Thresholds {
    /// Maximum allowed compute unit delta in absolute terms.
    pub max_cu_delta_abs: Option<u64>,
    /// Maximum allowed compute unit delta as a percentage (0-100).
    pub max_cu_delta_percent: Option<f32>,
    /// Whether to require exact program result parity (error code, index).
    pub require_result_parity: bool,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            max_cu_delta_abs: None,
            max_cu_delta_percent: None,
            require_result_parity: true,
        }
    }
}

/// Optional reporting configuration.
#[derive(Clone, Debug, Default)]
pub struct ReportConfig {
    /// Optional output directory for generated reports.
    pub out_dir: Option<std::path::PathBuf>,
    /// Emit markdown summary when true.
    pub markdown: bool,
    /// Emit JSON report when true.
    pub json: bool,
}

/// Builder for executing tests across multiple `FeatureSet` variants.
pub struct FeatureMatrix {
    /// Seed `Mollusk` instance provided by the caller.
    ///
    /// This allows callers to construct the matrix with a pre-configured
    /// harness (e.g., logging, budgets, or previously added programs) that
    /// the `setup` closure can reference or mirror as needed. The matrix
    /// currently builds fresh instances per variant to ensure clean state,
    /// but keeping the seed here documents intent and future-proofs APIs
    /// without impacting runtime behavior.
    _seed: Mollusk,
    baseline: BaselineConfig,
    variants: Vec<FeatureVariant>,
    thresholds: Thresholds,
    report: Option<ReportConfig>,
    resolver: Option<Box<dyn FeatureSetResolver + Send + Sync>>,
}

impl FeatureMatrix {
    /// Create a new matrix with a Mollusk instance and baseline config.
    pub fn new(mollusk: Mollusk, baseline: BaselineConfig) -> Self {
        Self {
            _seed: mollusk,
            baseline,
            variants: Vec::new(),
            thresholds: Thresholds::default(),
            report: None,
            resolver: None,
        }
    }

    /// Set or replace the baseline by slot.
    pub fn baseline_slot(mut self, slot: u64) -> Self {
        self.baseline = BaselineConfig::Slot(slot);
        self
    }

    /// Set or replace the baseline by explicit `FeatureSet`.
    pub fn baseline_featureset(mut self, features: FeatureSet) -> Self {
        self.baseline = BaselineConfig::Explicit(features);
        self
    }

    /// Add a single variant.
    pub fn variant(mut self, variant: FeatureVariant) -> Self {
        self.variants.push(variant);
        self
    }

    /// Replace variants.
    pub fn variants(mut self, variants: Vec<FeatureVariant>) -> Self {
        self.variants = variants;
        self
    }

    /// Configure thresholds.
    pub fn thresholds(mut self, thresholds: Thresholds) -> Self {
        self.thresholds = thresholds;
        self
    }

    /// Configure report output.
    pub fn report(mut self, report: ReportConfig) -> Self {
        self.report = Some(report);
        self
    }

    /// Finalize builder. Execution APIs will be added in subsequent steps.
    pub fn build(self) -> Self {
        self
    }

    /// Resolve the baseline `FeatureSet` based on configuration.
    pub fn resolve_baseline_featureset(&self) -> FeatureSet {
        match &self.baseline {
            BaselineConfig::Explicit(fs) => fs.clone(),
            BaselineConfig::Slot(slot) => self.resolve_featureset_for_slot(*slot),
        }
    }

    /// Apply a `FeatureVariant` on top of a `FeatureSet` and return the new set.
    pub fn apply_variant(&self, base: &FeatureSet, variant: &FeatureVariant) -> FeatureSet {
        if variant.enable.is_empty() {
            return base.clone();
        }

        let mut fs = base.clone();
        for feature_id in &variant.enable {
            fs.active_mut().insert(*feature_id, 0);
        }
        fs
    }

    fn resolve_featureset_for_slot(&self, slot: u64) -> FeatureSet {
        if let Some(resolver) = &self.resolver {
            return resolver.for_slot(slot);
        }
        // Default behavior: approximate with all-enabled, mirroring Mollusk::default
        // treatment of test-only features under the `fuzz` feature.
        #[cfg(feature = "fuzz")]
        {
            let mut fs = FeatureSet::all_enabled();
            fs.active_mut()
                .remove(&agave_feature_set::disable_sbpf_v0_execution::id());
            fs.active_mut()
                .remove(&agave_feature_set::reenable_sbpf_v0_execution::id());
            fs
        }
        #[cfg(not(feature = "fuzz"))]
        {
            let _ = slot; // unused without an external resolver
            FeatureSet::all_enabled()
        }
    }
}

/// Pluggable resolver to construct `FeatureSet` for a specific slot.
pub trait FeatureSetResolver {
    fn for_slot(&self, slot: u64) -> FeatureSet;
}

impl FeatureMatrix {
    pub fn with_resolver(mut self, resolver: Box<dyn FeatureSetResolver + Send + Sync>) -> Self {
        self.resolver = Some(resolver);
        self
    }
}

#[derive(Clone, Debug)]
pub struct VariantRun<T> {
    pub name: String,
    pub features: FeatureSet,
    pub output: T,
}

impl FeatureMatrix {
    pub fn build_mollusk_for_featureset(&self, features: &FeatureSet) -> Mollusk {
        let mut mollusk = Mollusk::default();
        mollusk.feature_set = features.clone();
        mollusk.program_cache = ProgramCache::new(&mollusk.feature_set, &mollusk.compute_budget);
        mollusk
    }

    /// Execute a user-provided setup and test against the baseline and all variants.
    ///
    /// `setup` should load programs and prepare any required state on the provided
    /// Mollusk instance (e.g., `add_program`, context wiring, etc.).
    /// `exec` runs the actual test/fixture and returns an output value.
    pub fn run_with_setup<T, S, E>(&self, setup: S, exec: E) -> Vec<VariantRun<T>>
    where
        S: Fn(&mut Mollusk),
        E: Fn(&mut Mollusk) -> T,
    {
        let mut results = Vec::with_capacity(1 + self.variants.len());

        // Baseline
        let baseline_features = self.resolve_baseline_featureset();
        let mut baseline_mollusk = self.build_mollusk_for_featureset(&baseline_features);
        setup(&mut baseline_mollusk);
        let baseline_output = exec(&mut baseline_mollusk);
        results.push(VariantRun {
            name: "baseline".to_string(),
            features: baseline_features.clone(),
            output: baseline_output,
        });

        // Variants
        for variant in &self.variants {
            let features = self.apply_variant(&baseline_features, variant);
            let mut mollusk = self.build_mollusk_for_featureset(&features);
            setup(&mut mollusk);
            let output = exec(&mut mollusk);
            results.push(VariantRun {
                name: if variant.name.is_empty() {
                    "variant".to_string()
                } else {
                    variant.name.clone()
                },
                features,
                output,
            });
        }

        results
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResultFieldPass {
    pub program_result: bool,
    pub return_data: bool,
    pub resulting_accounts: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct InstructionDiffEntry {
    pub variant: String,
    pub pass: bool,
    pub fields: ResultFieldPass,
}

impl FeatureMatrix {
    fn compare_check(
        &self,
        a: &result_model::InstructionResult,
        b: &result_model::InstructionResult,
        checks: &[result_model::compare::Compare],
    ) -> bool {
        a.compare_with_config(
            b,
            checks,
            &ResultConfig {
                panic: false,
                verbose: false,
            },
        )
    }

    /// Produce per-variant diffs against the baseline for core fields.
    ///
    /// Fields covered: program result, return data, resulting accounts.
    /// (Err index and logs are not part of `InstructionResult`; can be added via
    /// extended outputs in the caller if needed.)
    pub fn diff_instruction_results(
        &self,
        runs: &[VariantRun<result_model::InstructionResult>],
    ) -> Vec<InstructionDiffEntry> {
        if runs.is_empty() {
            return Vec::new();
        }
        let baseline = &runs[0].output;
        let mut entries = Vec::with_capacity(runs.len().saturating_sub(1));
        for run in runs.iter().skip(1) {
            let program_result = self.compare_check(
                baseline,
                &run.output,
                &[result_model::compare::Compare::ProgramResult],
            );
            let return_data = self.compare_check(
                baseline,
                &run.output,
                &[result_model::compare::Compare::ReturnData],
            );
            let resulting_accounts = self.compare_check(
                baseline,
                &run.output,
                &[result_model::compare::Compare::all_resulting_accounts()],
            );
            let fields = ResultFieldPass {
                program_result,
                return_data,
                resulting_accounts,
            };
            let pass = fields.program_result && fields.return_data && fields.resulting_accounts;
            entries.push(InstructionDiffEntry {
                variant: run.name.clone(),
                pass,
                fields,
            });
        }
        entries
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CuDeltaEntry {
    pub variant: String,
    pub baseline_cu: u64,
    pub variant_cu: u64,
    pub delta_abs: i64,
    pub delta_percent: Option<f32>,
    pub pass: bool,
}

impl FeatureMatrix {
    fn cu_passes_thresholds(&self, delta_abs: i64, delta_percent: Option<f32>) -> bool {
        let t = &self.thresholds;
        let mut pass = true;
        if let Some(max_abs) = t.max_cu_delta_abs {
            pass &= (delta_abs.unsigned_abs()) <= max_abs;
        }
        if let (Some(max_pct), Some(pct)) = (t.max_cu_delta_percent, delta_percent) {
            pass &= pct.abs() <= max_pct;
        }
        pass
    }

    /// Compute compute-unit deltas for each variant versus the baseline and
    /// mark pass/fail according to configured thresholds.
    pub fn cu_deltas(
        &self,
        runs: &[VariantRun<result_model::InstructionResult>],
    ) -> Vec<CuDeltaEntry> {
        if runs.is_empty() {
            return Vec::new();
        }
        let baseline_cu = runs[0].output.compute_units_consumed;
        let mut entries = Vec::with_capacity(runs.len().saturating_sub(1));
        for run in runs.iter().skip(1) {
            let variant_cu = run.output.compute_units_consumed;
            let delta_abs = variant_cu as i64 - baseline_cu as i64;
            let delta_percent = if baseline_cu == 0 {
                None
            } else {
                Some((delta_abs as f32) / (baseline_cu as f32) * 100.0)
            };
            let pass = self.cu_passes_thresholds(delta_abs, delta_percent);
            entries.push(CuDeltaEntry {
                variant: run.name.clone(),
                baseline_cu,
                variant_cu,
                delta_abs,
                delta_percent,
                pass,
            });
        }
        entries
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MatrixReportJson {
    pub baseline: String,
    pub diffs: Vec<InstructionDiffEntry>,
    pub cu: Vec<CuDeltaEntry>,
}

impl FeatureMatrix {
    fn render_markdown_table(headers: &[&str], rows: &[Vec<String>]) -> String {
        let mut out = String::new();
        out.push('|');
        for h in headers {
            out.push(' ');
            out.push_str(h);
            out.push_str(" |");
        }
        out.push('\n');
        out.push('|');
        for _ in headers {
            out.push_str("---|");
        }
        out.push('\n');
        // Rows
        for row in rows {
            out.push('|');
            for cell in row {
                out.push(' ');
                out.push_str(cell);
                out.push_str(" |");
            }
            out.push('\n');
        }
        out
    }

    fn build_markdown(
        &self,
        baseline_name: &str,
        diffs: &[InstructionDiffEntry],
        cu: &[CuDeltaEntry],
    ) -> String {
        let mut md = String::new();
        md.push_str("# Feature Matrix Report\n\n");
        md.push_str(&format!("Baseline: {}\n\n", baseline_name));

        // Diff summary
        md.push_str("## Result Parity\n\n");
        let diff_rows: Vec<Vec<String>> = diffs
            .iter()
            .map(|d| {
                vec![
                    d.variant.clone(),
                    if d.pass { "PASS".into() } else { "FAIL".into() },
                    if d.fields.program_result {
                        "OK".into()
                    } else {
                        "X".into()
                    },
                    if d.fields.return_data {
                        "OK".into()
                    } else {
                        "X".into()
                    },
                    if d.fields.resulting_accounts {
                        "OK".into()
                    } else {
                        "X".into()
                    },
                ]
            })
            .collect();
        md.push_str(&Self::render_markdown_table(
            &["Variant", "Pass", "ProgramResult", "ReturnData", "Accounts"],
            &diff_rows,
        ));
        md.push('\n');

        // CU summary
        md.push_str("## Compute Units\n\n");
        if let Some(first) = cu.first() {
            md.push_str(&format!("Baseline CU: {}\n\n", first.baseline_cu));
        }
        let cu_rows: Vec<Vec<String>> = cu
            .iter()
            .map(|e| {
                vec![
                    e.variant.clone(),
                    e.variant_cu.to_string(),
                    e.delta_abs.to_string(),
                    e.delta_percent
                        .map(|p| format!("{:.2}%", p))
                        .unwrap_or_else(|| "--".into()),
                    if e.pass { "PASS".into() } else { "FAIL".into() },
                ]
            })
            .collect();
        md.push_str(&Self::render_markdown_table(
            &["Variant", "CU", "Delta", "%", "Pass"],
            &cu_rows,
        ));
        md
    }

    /// Generate reports (Markdown/JSON) for a set of variant runs whose output is `InstructionResult`.
    /// Writes to `report.out_dir` when configured and returns the rendered content strings.
    pub fn generate_reports(
        &self,
        runs: &[VariantRun<result_model::InstructionResult>],
    ) -> (Option<String>, Option<String>) {
        if runs.is_empty() {
            return (None, None);
        }

        let baseline_name = runs[0].name.clone();
        let diffs = self.diff_instruction_results(runs);
        let cu = self.cu_deltas(runs);

        let json_obj = MatrixReportJson {
            baseline: baseline_name.clone(),
            diffs: diffs.clone(),
            cu: cu.clone(),
        };

        let markdown = if self.report.as_ref().map(|r| r.markdown).unwrap_or(false) {
            Some(self.build_markdown(&baseline_name, &diffs, &cu))
        } else {
            None
        };

        let json_string = if self.report.as_ref().map(|r| r.json).unwrap_or(false) {
            Some(serde_json::to_string_pretty(&json_obj).unwrap())
        } else {
            None
        };

        if let Some(cfg) = &self.report {
            if let Some(dir) = &cfg.out_dir {
                let _ = std::fs::create_dir_all(dir);
                if cfg.markdown {
                    if let Some(md) = &markdown {
                        let path = dir.join("feature_matrix.md");
                        let _ = std::fs::write(path, md);
                    }
                }
                if cfg.json {
                    if let Some(js) = &json_string {
                        let path = dir.join("feature_matrix.json");
                        let _ = std::fs::write(path, js);
                    }
                }
            }
        }

        (markdown, json_string)
    }
}
