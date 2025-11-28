#![cfg(any(feature = "fuzz", feature = "fuzz-fd"))]

use {
    agave_feature_set::FeatureSet,
    mollusk_svm::{result::Check, Mollusk},
    serial_test::serial,
    solana_account::Account,
    solana_instruction::Instruction,
    solana_pubkey::Pubkey,
    std::path::Path,
};

enum FileType {
    Blob,
    Json,
}

impl FileType {
    const fn extension(&self) -> &'static str {
        match self {
            Self::Blob => ".fix",
            Self::Json => ".json",
        }
    }
}

fn is_fixture_file(path: &Path, file_type: &FileType) -> bool {
    if path.is_file() {
        let path = path.to_str().unwrap();
        if path.ends_with(file_type.extension()) {
            return true;
        }
    }
    false
}

// Find the first fixture in the `EJECT_FUZZ_FIXTURES` directory.
fn find_fixture(dir: &str, file_type: &FileType) -> Option<String> {
    let dir = std::fs::read_dir(dir).unwrap();
    dir.filter_map(|entry| {
        let entry = entry.unwrap();
        let path = entry.path();
        if is_fixture_file(&path, file_type) {
            return Some(path.to_str().unwrap().to_string());
        }
        None
    })
    .next()
}

fn clear(dir: &str) {
    let _ = std::fs::remove_dir_all(dir);
}

fn compare_feature_sets(from_fixture: &FeatureSet, from_mollusk: &FeatureSet) {
    assert_eq!(from_fixture.active().len(), from_mollusk.active().len());
    assert_eq!(from_fixture.inactive().len(), from_mollusk.inactive().len());
    for f in from_fixture.active().keys() {
        assert!(from_mollusk.active().contains_key(f));
    }
}

fn assert_filenames_match(a: &str, b: &str) {
    assert_eq!(
        Path::new(a).file_stem().unwrap().to_str().unwrap(),
        Path::new(b).file_stem().unwrap().to_str().unwrap()
    );
}

const BASE_LAMPORTS: u64 = 100_000_000;
const TRANSFER_AMOUNT: u64 = 42_000;

struct TestSetup<'a> {
    mollusk: Mollusk,
    instruction: Instruction,
    accounts: Vec<(Pubkey, Account)>,
    checks: Vec<Check<'a>>,
}

impl<'a> TestSetup<'a> {
    fn new(sender: &'a Pubkey, recipient: &'a Pubkey) -> Self {
        let mollusk = Mollusk::default();

        let instruction =
            solana_system_interface::instruction::transfer(sender, recipient, TRANSFER_AMOUNT);
        let accounts = vec![
            (
                *sender,
                Account::new(BASE_LAMPORTS, 0, &solana_sdk_ids::system_program::id()),
            ),
            (
                *recipient,
                Account::new(BASE_LAMPORTS, 0, &solana_sdk_ids::system_program::id()),
            ),
        ];
        let checks = vec![
            Check::success(),
            Check::account(sender)
                .lamports(BASE_LAMPORTS - TRANSFER_AMOUNT)
                .build(),
            Check::account(recipient)
                .lamports(BASE_LAMPORTS + TRANSFER_AMOUNT)
                .build(),
        ];

        Self {
            mollusk,
            instruction,
            accounts,
            checks,
        }
    }

    #[cfg(feature = "fuzz")]
    fn check_fixture_mollusk(&self, fixture: mollusk_svm_fuzz_fixture::Fixture) {
        assert_eq!(fixture.input.compute_budget, self.mollusk.compute_budget);
        // Feature set matches, but it can't guarantee sorting.
        compare_feature_sets(&fixture.input.feature_set, &self.mollusk.feature_set);
        assert_eq!(fixture.input.sysvars.clock, self.mollusk.sysvars.clock);
        assert_eq!(fixture.input.sysvars.rent, self.mollusk.sysvars.rent);
        assert_eq!(fixture.input.program_id, self.instruction.program_id);
        assert_eq!(
            fixture.input.instruction_accounts,
            self.instruction.accounts
        );
        assert_eq!(fixture.input.instruction_data, self.instruction.data);
        assert_eq!(fixture.input.accounts, self.accounts);
    }

    #[cfg(feature = "fuzz-fd")]
    fn check_fixture_firedancer(
        &self,
        fixture: mollusk_svm_fuzz_fixture_firedancer::Fixture,
        result: &mollusk_svm::result::InstructionResult,
    ) {
        // Inputs:
        assert_eq!(fixture.input.program_id, self.instruction.program_id);
        // I'm sure accounts are fine...
        assert_eq!(fixture.input.instruction_data, self.instruction.data);
        assert_eq!(
            fixture.input.compute_units_available,
            self.mollusk.compute_budget.compute_unit_limit,
        );
        assert_eq!(
            fixture.input.slot_context.slot,
            self.mollusk.sysvars.clock.slot,
        );
        // Feature set matches, but it can't guarantee sorting.
        compare_feature_sets(
            &fixture.input.epoch_context.feature_set,
            &self.mollusk.feature_set,
        );
        // Outputs:
        assert_eq!(
            fixture.output.compute_units_available,
            self.mollusk
                .compute_budget
                .compute_unit_limit
                .saturating_sub(result.compute_units_consumed),
        );
    }
}

#[cfg(feature = "fuzz")]
#[test]
#[serial]
fn test_dump_mollusk() {
    use mollusk_svm_fuzz_fixture::Fixture;

    const EJECT_FUZZ_FIXTURES: &str = "./tests/mollusk-fixtures";

    clear(EJECT_FUZZ_FIXTURES);
    std::env::set_var("EJECT_FUZZ_FIXTURES", EJECT_FUZZ_FIXTURES);
    std::env::set_var("EJECT_FUZZ_FIXTURES_JSON", EJECT_FUZZ_FIXTURES);

    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let setup = TestSetup::new(&sender, &recipient);

    setup.mollusk.process_and_validate_instruction(
        &setup.instruction,
        &setup.accounts,
        &setup.checks,
    );

    // Validate the protobuf fixture matches the test environment.
    let blob_fixture_path = find_fixture(EJECT_FUZZ_FIXTURES, &FileType::Blob).unwrap();
    let blob_fixture = Fixture::load_from_blob_file(&blob_fixture_path);
    setup.check_fixture_mollusk(blob_fixture);

    // Validate the JSON fixture matches the test environment.
    let json_fixture_path = find_fixture(EJECT_FUZZ_FIXTURES, &FileType::Json).unwrap();
    let json_fixture = Fixture::load_from_json_file(&json_fixture_path);
    setup.check_fixture_mollusk(json_fixture);

    // Ensure both files have the same name.
    assert_filenames_match(&blob_fixture_path, &json_fixture_path);

    // Now check instruction chains.
    clear(EJECT_FUZZ_FIXTURES);

    setup.mollusk.process_and_validate_instruction_chain(
        &[
            (&setup.instruction, &[]),
            (&setup.instruction, &[]),
            (&setup.instruction, &[]),
        ],
        &setup.accounts,
    );

    // Ensure there are three of each fixture type in the target directory.
    let dir = std::fs::read_dir(EJECT_FUZZ_FIXTURES).unwrap();
    let mut count_blob = 0;
    let mut count_json = 0;
    for entry in dir {
        let path = entry.unwrap().path();
        if is_fixture_file(&path, &FileType::Blob) {
            count_blob += 1;
        }
        if is_fixture_file(&path, &FileType::Json) {
            count_json += 1;
        }
    }
    assert_eq!(count_blob, 3);
    assert_eq!(count_json, 3);

    std::env::remove_var("EJECT_FUZZ_FIXTURES");
    std::env::remove_var("EJECT_FUZZ_FIXTURES_JSON");
    clear(EJECT_FUZZ_FIXTURES);
}

#[cfg(feature = "fuzz-fd")]
#[test]
#[serial]
fn test_dump_firedancer() {
    use mollusk_svm_fuzz_fixture_firedancer::Fixture;

    const EJECT_FUZZ_FIXTURES_FD: &str = "./tests/firedancer-fixtures";

    clear(EJECT_FUZZ_FIXTURES_FD);
    std::env::set_var("EJECT_FUZZ_FIXTURES_FD", EJECT_FUZZ_FIXTURES_FD);
    std::env::set_var("EJECT_FUZZ_FIXTURES_JSON_FD", EJECT_FUZZ_FIXTURES_FD);

    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let setup = TestSetup::new(&sender, &recipient);

    let result = setup.mollusk.process_and_validate_instruction(
        &setup.instruction,
        &setup.accounts,
        &setup.checks,
    );

    // Validate the protobuf fixture matches the test environment.
    let blob_fixture_path = find_fixture(EJECT_FUZZ_FIXTURES_FD, &FileType::Blob).unwrap();
    let blob_fixture = Fixture::load_from_blob_file(&blob_fixture_path);
    setup.check_fixture_firedancer(blob_fixture, &result);

    // Validate the JSON fixture matches the test environment.
    let json_fixture_path = find_fixture(EJECT_FUZZ_FIXTURES_FD, &FileType::Json).unwrap();
    let json_fixture = Fixture::load_from_json_file(&json_fixture_path);
    setup.check_fixture_firedancer(json_fixture, &result);

    // Ensure both files have the same name.
    assert_filenames_match(&blob_fixture_path, &json_fixture_path);

    // Now check instruction chains.
    clear(EJECT_FUZZ_FIXTURES_FD);

    setup.mollusk.process_and_validate_instruction_chain(
        &[
            (&setup.instruction, &[]),
            (&setup.instruction, &[]),
            (&setup.instruction, &[]),
        ],
        &setup.accounts,
    );

    // Ensure there are three of each fixture type in the target directory.
    let dir = std::fs::read_dir(EJECT_FUZZ_FIXTURES_FD).unwrap();
    let mut count_blob = 0;
    let mut count_json = 0;
    for entry in dir {
        let path = entry.unwrap().path();
        if is_fixture_file(&path, &FileType::Blob) {
            count_blob += 1;
        }
        if is_fixture_file(&path, &FileType::Json) {
            count_json += 1;
        }
    }
    assert_eq!(count_blob, 3);
    assert_eq!(count_json, 3);

    std::env::remove_var("EJECT_FUZZ_FIXTURES_FD");
    std::env::remove_var("EJECT_FUZZ_FIXTURES_JSON_FD");
    clear(EJECT_FUZZ_FIXTURES_FD);
}
