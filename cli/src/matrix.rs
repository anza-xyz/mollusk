//! Feature-activation matrix runner for testing program behavior across
//! different FeatureSet configurations. CLI-only implementation.

use {
    agave_feature_set::FeatureSet,
    mollusk_svm::{program::ProgramCache, result as result_model, Mollusk},
};

/// Baseline selector for feature-activation comparisons.
#[derive(Clone, Debug)]
pub enum BaselineConfig {
    /// Use an explicit `FeatureSet`.
    Explicit(FeatureSet),
}

/// A named variant of features to enable for comparison.
#[derive(Clone, Debug, Default)]
pub struct FeatureVariant {
    /// Human-friendly name for reports.
    pub name: String,
    /// Features to explicitly enable on top of the baseline.
    pub enable: Vec<solana_pubkey::Pubkey>,
}

/// Thresholds used to flag regressions in comparisons.
#[derive(Clone, Debug, Default)]
pub struct Thresholds {
    /// Maximum allowed compute unit delta in absolute terms.
    pub max_cu_delta_abs: Option<u64>,
    /// Maximum allowed compute unit delta as a percentage (0-100).
    pub max_cu_delta_percent: Option<f32>,
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
    _seed: Mollusk,
    baseline: BaselineConfig,
    thresholds: Thresholds,
    report: Option<ReportConfig>,
}

impl FeatureMatrix {
    pub fn new(mollusk: Mollusk, baseline: BaselineConfig) -> Self {
        Self {
            _seed: mollusk,
            baseline,
            thresholds: Thresholds::default(),
            report: None,
        }
    }

    pub fn thresholds(mut self, thresholds: Thresholds) -> Self {
        self.thresholds = thresholds;
        self
    }

    pub fn report(mut self, report: ReportConfig) -> Self {
        self.report = Some(report);
        self
    }

    pub fn build(self) -> Self { self }

    pub fn resolve_baseline_featureset(&self) -> FeatureSet {
        match &self.baseline {
            BaselineConfig::Explicit(fs) => fs.clone(),
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

    pub fn build_mollusk_for_featureset(&self, features: &FeatureSet) -> Mollusk {
        let mut mollusk = Mollusk::default();
        mollusk.feature_set = features.clone();
        mollusk.program_cache = ProgramCache::new(&mollusk.feature_set, &mollusk.compute_budget);
        mollusk
    }
}

#[derive(Clone, Debug)]
pub struct VariantRun<T> {
    pub name: String,
    pub output: T,
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
            &mollusk_svm::result::Config { panic: false, verbose: false },
        )
    }

    pub fn diff_instruction_results(
        &self,
        runs: &[VariantRun<result_model::InstructionResult>],
    ) -> Vec<InstructionDiffEntry> {
        if runs.is_empty() { return Vec::new(); }
        let baseline = &runs[0].output;
        let mut entries = Vec::with_capacity(runs.len().saturating_sub(1));
        for run in runs.iter().skip(1) {
            let program_result = self.compare_check(baseline, &run.output, &[result_model::compare::Compare::ProgramResult]);
            let return_data = self.compare_check(baseline, &run.output, &[result_model::compare::Compare::ReturnData]);
            let resulting_accounts = self.compare_check(baseline, &run.output, &[result_model::compare::Compare::all_resulting_accounts()]);
            let fields = ResultFieldPass { program_result, return_data, resulting_accounts };
            let pass = fields.program_result && fields.return_data && fields.resulting_accounts;
            entries.push(InstructionDiffEntry { variant: run.name.clone(), pass, fields });
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
        if let Some(max_abs) = t.max_cu_delta_abs { pass &= (delta_abs.unsigned_abs()) <= max_abs; }
        if let (Some(max_pct), Some(pct)) = (t.max_cu_delta_percent, delta_percent) { pass &= pct.abs() <= max_pct; }
        pass
    }

    pub fn cu_deltas(
        &self,
        runs: &[VariantRun<result_model::InstructionResult>],
    ) -> Vec<CuDeltaEntry> {
        if runs.is_empty() { return Vec::new(); }
        let baseline_cu = runs[0].output.compute_units_consumed;
        let mut entries = Vec::with_capacity(runs.len().saturating_sub(1));
        for run in runs.iter().skip(1) {
            let variant_cu = run.output.compute_units_consumed;
            let delta_abs = variant_cu as i64 - baseline_cu as i64;
            let delta_percent = if baseline_cu == 0 { None } else { Some((delta_abs as f32) / (baseline_cu as f32) * 100.0) };
            let pass = self.cu_passes_thresholds(delta_abs, delta_percent);
            entries.push(CuDeltaEntry { variant: run.name.clone(), baseline_cu, variant_cu, delta_abs, delta_percent, pass });
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
        for h in headers { out.push(' '); out.push_str(h); out.push_str(" |"); }
        out.push('\n');
        out.push('|');
        for _ in headers { out.push_str("---|"); }
        out.push('\n');
        for row in rows {
            out.push('|');
            for cell in row { out.push(' '); out.push_str(cell); out.push_str(" |"); }
            out.push('\n');
        }
        out
    }

    fn build_markdown(&self, baseline_name: &str, diffs: &[InstructionDiffEntry], cu: &[CuDeltaEntry]) -> String {
        let mut md = String::new();
        md.push_str("# Feature Matrix Report\n\n");
        md.push_str(&format!("Baseline: {}\n\n", baseline_name));
        md.push_str("## Result Parity\n\n");
        let diff_rows: Vec<Vec<String>> = diffs.iter().map(|d| vec![
            d.variant.clone(),
            if d.pass { "PASS".into() } else { "FAIL".into() },
            if d.fields.program_result { "OK".into() } else { "X".into() },
            if d.fields.return_data { "OK".into() } else { "X".into() },
            if d.fields.resulting_accounts { "OK".into() } else { "X".into() },
        ]).collect();
        md.push_str(&Self::render_markdown_table(&["Variant", "Pass", "ProgramResult", "ReturnData", "Accounts"], &diff_rows));
        md.push('\n');
        md.push_str("## Compute Units\n\n");
        if let Some(first) = cu.first() { md.push_str(&format!("Baseline CU: {}\n\n", first.baseline_cu)); }
        let cu_rows: Vec<Vec<String>> = cu.iter().map(|e| vec![
            e.variant.clone(),
            e.variant_cu.to_string(),
            e.delta_abs.to_string(),
            e.delta_percent.map(|p| format!("{:.2}%", p)).unwrap_or_else(|| "--".into()),
            if e.pass { "PASS".into() } else { "FAIL".into() },
        ]).collect();
        md.push_str(&Self::render_markdown_table(&["Variant", "CU", "Delta", "%", "Pass"], &cu_rows));
        md
    }

    /// Generate reports (Markdown/JSON) for a set of variant runs whose output
    /// is `InstructionResult`. Writes to `report.out_dir` when configured
    /// and returns the rendered content strings.
    pub fn generate_reports(
        &self,
        runs: &[VariantRun<result_model::InstructionResult>],
    ) -> Result<(Option<String>, Option<String>), Box<dyn std::error::Error>> {
        if runs.is_empty() { return Ok((None, None)); }

        let baseline_name = runs[0].name.clone();
        let diffs = self.diff_instruction_results(runs);
        let cu = self.cu_deltas(runs);

        let json_obj = MatrixReportJson { baseline: baseline_name.clone(), diffs: diffs.clone(), cu: cu.clone() };

        let markdown = if self.report.as_ref().map(|r| r.markdown).unwrap_or(false) {
            Some(self.build_markdown(&baseline_name, &diffs, &cu))
        } else { None };

        let json_string = if self.report.as_ref().map(|r| r.json).unwrap_or(false) {
            Some(serde_json::to_string_pretty(&json_obj).unwrap())
        } else { None };

        if let Some(cfg) = &self.report {
            if let Some(dir) = &cfg.out_dir {
                std::fs::create_dir_all(dir)
                    .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("failed to create out_dir: {}", e))) as Box<dyn std::error::Error>)?;
                if cfg.markdown {
                    if let Some(md) = &markdown {
                        let path = dir.join("feature_matrix.md");
                        std::fs::write(&path, md).map_err(|e| {
                            Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!(
                                "failed to write {:?}: {}", path, e
                            ))) as Box<dyn std::error::Error>
                        })?;
                    }
                }
                if cfg.json {
                    if let Some(js) = &json_string {
                        let path = dir.join("feature_matrix.json");
                        std::fs::write(&path, js).map_err(|e| {
                            Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!(
                                "failed to write {:?}: {}", path, e
                            ))) as Box<dyn std::error::Error>
                        })?;
                    }
                }
            }
        }

        Ok((markdown, json_string))
    }
}


