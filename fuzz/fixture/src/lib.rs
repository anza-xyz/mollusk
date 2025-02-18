//! Mollusk SVM Fuzz: Mollusk-compatible fuzz fixture for SVM programs.
//!
//! Note: The fixtures defined in this library are compatible with Mollusk,
//! which means developers can fuzz programs using the Mollusk harness.
//! However, these fixtures (and this library) do not depend on the harness.
//! They can be used to fuzz a custom entrypoint of the developer's choice.

pub mod account;
pub mod compute_budget;
pub mod context;
pub mod effects;
pub mod feature_set;
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/org.mollusk.svm.rs"));
}
pub mod sysvars;

use {
    crate::{context::Context, effects::Effects, proto::InstrFixture as ProtoFixture},
    mollusk_svm_fuzz_fs::{FsHandler, IntoSerializableFixture, SerializableFixture},
    solana_keccak_hasher::{Hash, Hasher},
};

/// A fixture for invoking a single instruction against a simulated SVM
/// program runtime environment, for a given program.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fixture {
    /// The fixture inputs.
    pub input: Context,
    /// The fixture outputs.
    pub output: Effects,
}

impl Fixture {
    pub fn decode(blob: &[u8]) -> Self {
        let proto_fixture = <ProtoFixture as SerializableFixture>::decode(blob);
        proto_fixture.into()
    }

    pub fn load_from_blob_file(file_path: &str) -> Self {
        let proto_fixture: ProtoFixture = FsHandler::load_from_blob_file(file_path);
        proto_fixture.into()
    }

    pub fn load_from_json_file(file_path: &str) -> Self {
        let proto_fixture: ProtoFixture = FsHandler::load_from_json_file(file_path);
        proto_fixture.into()
    }
}

impl From<ProtoFixture> for Fixture {
    fn from(value: ProtoFixture) -> Self {
        // All blobs should have an input and output.
        Self {
            input: value.input.unwrap().into(),
            output: value.output.unwrap().into(),
        }
    }
}

impl From<Fixture> for ProtoFixture {
    fn from(value: Fixture) -> Self {
        Self {
            input: Some(value.input.into()),
            output: Some(value.output.into()),
        }
    }
}

impl SerializableFixture for ProtoFixture {
    // Manually implemented for deterministic hashes.
    fn hash(&self) -> Hash {
        let mut hasher = Hasher::default();
        if let Some(input) = &self.input {
            crate::context::hash_proto_context(&mut hasher, input);
        }
        if let Some(output) = &self.output {
            crate::effects::hash_proto_effects(&mut hasher, output);
        }
        hasher.result()
    }
}

impl IntoSerializableFixture for Fixture {
    type Fixture = ProtoFixture;

    fn into(self) -> Self::Fixture {
        Into::into(self)
    }
}

#[cfg(test)]
mod tests {
    use {
        super::{proto::InstrFixture, Fixture},
        crate::{context::Context, effects::Effects, sysvars::Sysvars},
        mollusk_svm_fuzz_fs::SerializableFixture,
        solana_account::Account,
        solana_compute_budget::compute_budget::ComputeBudget,
        solana_feature_set::FeatureSet,
        solana_instruction::AccountMeta,
        solana_keccak_hasher::Hash,
        solana_pubkey::Pubkey,
    };

    fn produce_hash(fixture: &Fixture) -> Hash {
        let proto_fixture: InstrFixture = fixture.clone().into();
        proto_fixture.hash()
    }

    #[test]
    fn test_consistent_hashing() {
        const ITERATIONS: usize = 1000;

        let compute_budget = ComputeBudget::default();
        let feature_set = FeatureSet::all_enabled();
        let sysvars = Sysvars::default();
        let program_id = Pubkey::default();
        let instruction_accounts = vec![
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new(Pubkey::new_unique(), false),
        ];
        let instruction_data = vec![4; 24];
        let accounts = instruction_accounts
            .iter()
            .map(|meta| (meta.pubkey, Account::new(42, 42, &Pubkey::default())))
            .collect::<Vec<_>>();

        let context = Context {
            compute_budget,
            feature_set,
            sysvars,
            program_id,
            instruction_accounts,
            instruction_data,
            accounts,
        };
        let effects = Effects::default();

        let fixture = Fixture {
            input: context,
            output: effects,
        };

        let mut last_hash = produce_hash(&fixture);
        for _ in 0..ITERATIONS {
            let new_hash = produce_hash(&fixture);
            assert_eq!(last_hash, new_hash);
            last_hash = new_hash;
        }
    }
}
