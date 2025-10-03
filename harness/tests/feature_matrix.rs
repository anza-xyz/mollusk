use {
    agave_feature_set::FeatureSet,
    mollusk_svm::{
        feature_matrix::{
            BaselineConfig, CuDeltaEntry, FeatureMatrix, FeatureVariant, ReportConfig,
            ResultFieldPass, VariantRun,
        },
        result::InstructionResult,
        Mollusk,
    },
    solana_account::Account,
    solana_pubkey::Pubkey,
};

fn instr_result_with(
    compute_units_consumed: u64,
    return_data: &[u8],
    accounts: &[(Pubkey, Account)],
) -> InstructionResult {
    InstructionResult {
        compute_units_consumed,
        return_data: return_data.to_vec(),
        resulting_accounts: accounts.to_vec(),
        ..Default::default()
    }
}

#[test]
fn test_featureset_apply_variant_enables_ids() {
    let mollusk = Mollusk::default();
    let baseline = BaselineConfig::Explicit(FeatureSet::default());
    let fm = FeatureMatrix::new(mollusk, baseline);

    // Create a random feature id to enable (not asserting semantics, only
    // insertion).
    let fid = Pubkey::new_unique();
    let base = FeatureSet::default();
    assert!(!base.is_active(&fid));
    let v = FeatureVariant {
        name: "v1".into(),
        enable: vec![fid],
    };
    let applied = fm.apply_variant(&base, &v);
    assert!(applied.is_active(&fid));

    let fs_slot = fm.resolve_baseline_featureset();
    let _ = fs_slot.runtime_features();
}

#[test]
fn test_diff_instruction_results_pass_and_fail() {
    let mollusk = Mollusk::default();
    let fm = FeatureMatrix::new(mollusk, BaselineConfig::Explicit(FeatureSet::default()));

    let a0 = (Pubkey::new_unique(), Account::default());
    let a1 = (Pubkey::new_unique(), Account::default());

    let baseline = instr_result_with(100, b"ok", &[a0.clone(), a1.clone()]);
    let identical = instr_result_with(100, b"ok", &[a0.clone(), a1.clone()]);
    let different = instr_result_with(120, b"nope", &[a0.clone(), a1.clone()]);

    let runs = vec![
        VariantRun {
            name: "baseline".into(),
            features: FeatureSet::default(),
            output: baseline,
        },
        VariantRun {
            name: "identical".into(),
            features: FeatureSet::default(),
            output: identical,
        },
        VariantRun {
            name: "different".into(),
            features: FeatureSet::default(),
            output: different,
        },
    ];

    let diffs = fm.diff_instruction_results(&runs);
    assert_eq!(diffs.len(), 2);
    let d0 = &diffs[0];
    assert_eq!(d0.variant, "identical");
    assert!(d0.pass);
    assert_eq!(
        d0.fields,
        ResultFieldPass {
            program_result: true,
            return_data: true,
            resulting_accounts: true
        }
    );
    let d1 = &diffs[1];
    assert_eq!(d1.variant, "different");
    assert!(!d1.pass);
    assert!(!d1.fields.return_data);
}

#[test]
fn test_cu_deltas_and_reports_golden() {
    let mollusk = Mollusk::default();
    let mut fm = FeatureMatrix::new(mollusk, BaselineConfig::Explicit(FeatureSet::default()));

    // Build runs with clear CU delta and identical results for parity.
    let pk = Pubkey::new_unique();
    let acc = Account::default();
    let baseline = instr_result_with(1000, b"", &[(pk, acc.clone())]);
    let v1 = instr_result_with(1100, b"", &[(pk, acc.clone())]);

    let runs = vec![
        VariantRun {
            name: "baseline".into(),
            features: FeatureSet::default(),
            output: baseline,
        },
        VariantRun {
            name: "v1".into(),
            features: FeatureSet::default(),
            output: v1,
        },
    ];

    // Thresholds allow up to 15% increase.
    fm = fm.thresholds(mollusk_svm::feature_matrix::Thresholds {
        max_cu_delta_abs: None,
        max_cu_delta_percent: Some(15.0),
        require_result_parity: true,
    });

    let cu = fm.cu_deltas(&runs);
    assert_eq!(cu.len(), 1);
    let CuDeltaEntry {
        variant,
        baseline_cu,
        variant_cu,
        delta_abs,
        delta_percent,
        pass,
    } = &cu[0];
    assert_eq!(variant, "v1");
    assert_eq!(*baseline_cu, 1000);
    assert_eq!(*variant_cu, 1100);
    assert_eq!(*delta_abs, 100);
    assert!(delta_percent.is_some());
    assert!(*pass);

    let fm = fm
        .report(ReportConfig {
            out_dir: None,
            markdown: true,
            json: true,
        })
        .build();
    let (md, js) = fm.generate_reports(&runs);
    let md = md.expect("markdown");
    let js = js.expect("json");

    // Golden snippets
    assert!(md.contains("# Feature Matrix Report"));
    assert!(md.contains("## Result Parity"));
    assert!(md.contains("## Compute Units"));
    assert!(js.contains("\"baseline\""));
    assert!(js.contains("\"diffs\""));
    assert!(js.contains("\"cu\""));
}
