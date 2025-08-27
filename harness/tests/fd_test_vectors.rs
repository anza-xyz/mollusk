#![cfg(feature = "fuzz-fd")]

use {
    agave_feature_set::FeatureSet,
    mollusk_svm::{
        fuzz::firedancer::{
            build_fixture_from_mollusk_test, load_firedancer_fixture, ParsedFixtureContext,
        },
        Mollusk,
    },
    mollusk_svm_fuzz_fixture_firedancer::{account::SeedAddress, Fixture},
    rayon::prelude::*,
    solana_account::Account,
    solana_pubkey::Pubkey,
    solana_transaction_context::InstructionAccount,
    std::{assert_eq, fs, path::Path, process::Command},
};

const TEST_VECTORS_PATH: &str = "tests/test-vectors";
const TEST_VECTORS_REPOSITORY: &str = "https://github.com/buffalojoec/test-vectors.git";
const TEST_VECTORS_BRANCH: &str = "mollusk-tests";
const TEST_VECTORS_TO_TEST: &[&str] = &[
    "instr/fixtures/address-lookup-table",
    "instr/fixtures/config",
    "instr/fixtures/stake",
    // Add more here!
];

#[test]
fn test_load_firedancer_fixtures() {
    let test_vectors_out_dir = Path::new(TEST_VECTORS_PATH);

    // Fetch the test vectors.
    if !test_vectors_out_dir.exists() {
        Command::new("git")
            .arg("clone")
            .arg("--depth=1")
            .arg("--branch")
            .arg(TEST_VECTORS_BRANCH)
            .arg(TEST_VECTORS_REPOSITORY)
            .arg(test_vectors_out_dir)
            .status()
            .expect("Failed to execute git clone");
    }

    // Attempt to go fixture -> Mollusk -> fixture and compare.
    TEST_VECTORS_TO_TEST.par_iter().for_each(|directory| {
        fs::read_dir(test_vectors_out_dir.join(directory))
            .unwrap()
            .par_bridge()
            .for_each(|entry| {
                let path = entry.unwrap().path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "fix") {
                    let loaded_fixture = Fixture::load_from_blob_file(path.to_str().unwrap());
                    let (
                        ParsedFixtureContext {
                            accounts,
                            compute_budget,
                            feature_set,
                            instruction,
                            slot,
                        },
                        result,
                    ) = load_firedancer_fixture(&loaded_fixture);
                    let mollusk = Mollusk {
                        compute_budget,
                        feature_set,
                        slot,
                        ..Default::default()
                    };
                    let generated_fixture =
                        build_fixture_from_mollusk_test(&mollusk, &instruction, &accounts, &result);

                    assert_eq!(loaded_fixture.metadata, generated_fixture.metadata);
                    assert_eq!(
                        loaded_fixture.input.program_id,
                        generated_fixture.input.program_id,
                    );
                    // Sometimes ordering is not the same because of the `KeyMap`.
                    // Contents should match though.
                    compare_accounts(
                        &loaded_fixture.input.accounts,
                        &generated_fixture.input.accounts,
                    );
                    compare_instruction_accounts(
                        &loaded_fixture.input.instruction_accounts,
                        &generated_fixture.input.instruction_accounts,
                    );
                    assert_eq!(
                        loaded_fixture.input.compute_units_available,
                        generated_fixture.input.compute_units_available,
                    );
                    assert_eq!(
                        loaded_fixture.input.slot_context,
                        generated_fixture.input.slot_context,
                    );
                    // Feature set is not always ordered the same as a side effect
                    // of `HashMap`.
                    compare_feature_sets(
                        &loaded_fixture.input.epoch_context.feature_set,
                        &generated_fixture.input.epoch_context.feature_set,
                    );
                    assert_eq!(
                        loaded_fixture.output.program_result,
                        generated_fixture.output.program_result,
                    );
                    assert_eq!(
                        loaded_fixture.output.program_custom_code,
                        generated_fixture.output.program_custom_code,
                    );
                    compare_accounts(
                        &loaded_fixture.output.modified_accounts,
                        &generated_fixture.output.modified_accounts,
                    );
                    assert_eq!(
                        loaded_fixture.output.compute_units_available,
                        generated_fixture.output.compute_units_available,
                    );
                    assert_eq!(
                        loaded_fixture.output.return_data,
                        generated_fixture.output.return_data,
                    );
                }
            });
    });
}

fn compare_accounts(
    a: &[(Pubkey, Account, Option<SeedAddress>)],
    b: &[(Pubkey, Account, Option<SeedAddress>)],
) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut a_sorted = a.to_vec();
    let mut b_sorted = b.to_vec();

    // Sort by Pubkey
    a_sorted.sort_by(|(pubkey_a, _, _), (pubkey_b, _, _)| pubkey_a.cmp(pubkey_b));
    b_sorted.sort_by(|(pubkey_a, _, _), (pubkey_b, _, _)| pubkey_a.cmp(pubkey_b));

    // Compare sorted lists
    a_sorted == b_sorted
}

fn compare_instruction_accounts(a: &[InstructionAccount], b: &[InstructionAccount]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut a_sorted = a.to_vec();
    let mut b_sorted = b.to_vec();

    // Sort by index_in_transaction
    a_sorted.sort_by(|ia_a, ia_b| ia_a.index_in_transaction.cmp(&ia_b.index_in_transaction));
    b_sorted.sort_by(|ia_a, ia_b| ia_a.index_in_transaction.cmp(&ia_b.index_in_transaction));

    // Compare sorted lists; InstructionAccount no longer implements PartialEq
    a_sorted
        .iter()
        .zip(b_sorted.iter())
        .all(|(a_item, b_item)| {
            a_item.index_in_transaction == b_item.index_in_transaction
                && a_item.is_signer() == b_item.is_signer()
                && a_item.is_writable() == b_item.is_writable()
        })
}

fn compare_feature_sets(from_fixture: &FeatureSet, from_mollusk: &FeatureSet) {
    assert_eq!(from_fixture.active().len(), from_mollusk.active().len());
    assert_eq!(from_fixture.inactive().len(), from_mollusk.inactive().len());
    for f in from_fixture.active().keys() {
        assert!(from_mollusk.active().contains_key(f));
    }
}
