//! Tests for ABIv2 features.

use mollusk_svm::Mollusk;

#[test]
fn test_program_runtime_abiv2_feature_is_active_by_default() {
    let mollusk = Mollusk::default();
    assert!(mollusk.feature_set.program_runtime_abiv2);
}
