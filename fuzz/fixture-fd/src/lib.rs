//! Mollusk SVM Fuzz: Mollusk-compatible Firedancer fuzz fixture for SVM
//! programs.
//!
//! Similar to the `mollusk-svm-fuzz-fixture` library, but built around
//! Firedancer's protobuf layouts.
//!
//! Note: The fixtures defined in this library are compatible with Mollusk,
//! which means developers can fuzz programs using the Mollusk harness.
//! However, these fixtures (and this library) do not depend on the harness.
//! They can be used to fuzz a custom entrypoint of the developer's choice.

pub mod account;
pub mod context;
pub mod effects;
pub mod feature_set;
pub mod instr_account;
pub mod metadata;
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/org.solana.sealevel.v1.rs"));
}

use {
    crate::{
        context::Context, effects::Effects, metadata::Metadata, proto::InstrFixture as ProtoFixture,
    },
    mollusk_svm_fuzz_fs::{FsHandler, IntoSerializableFixture, SerializableFixture},
    solana_keccak_hasher::{Hash, Hasher},
};

/// A fixture for invoking a single instruction against a simulated SVM
/// program runtime environment, for a given program.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fixture {
    /// The fixture metadata.
    pub metadata: Option<Metadata>,
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
            metadata: value.metadata.map(Into::into),
            input: value.input.unwrap().into(),
            output: value.output.unwrap().into(),
        }
    }
}

impl From<Fixture> for ProtoFixture {
    fn from(value: Fixture) -> Self {
        Self {
            metadata: value.metadata.map(Into::into),
            input: Some(value.input.into()),
            output: Some(value.output.into()),
        }
    }
}

impl SerializableFixture for ProtoFixture {
    // Manually implemented for deterministic hashes.
    fn hash(&self) -> Hash {
        let mut hasher = Hasher::default();
        if let Some(metadata) = &self.metadata {
            crate::metadata::hash_proto_metadata(&mut hasher, metadata);
        }
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
        crate::{
            context::{Context, EpochContext, SlotContext},
            effects::Effects,
            metadata::Metadata,
        },
        mollusk_svm_fuzz_fs::SerializableFixture,
        solana_account::Account,
        solana_feature_set::FeatureSet,
        solana_keccak_hasher::Hash,
        solana_pubkey::Pubkey,
        solana_transaction_context::InstructionAccount,
    };

    fn produce_hash(fixture: &Fixture) -> Hash {
        let proto_fixture: InstrFixture = fixture.clone().into();
        proto_fixture.hash()
    }

    #[test]
    fn test_consistent_hashing() {
        const ITERATIONS: usize = 1000;

        let program_id = Pubkey::default();
        let accounts = vec![
            (
                Pubkey::new_unique(),
                Account::new(42, 42, &Pubkey::default()),
                None,
            ),
            (
                Pubkey::new_unique(),
                Account::new(42, 42, &Pubkey::default()),
                None,
            ),
            (
                Pubkey::new_unique(),
                Account::new(42, 42, &Pubkey::default()),
                None,
            ),
            (
                Pubkey::new_unique(),
                Account::new(42, 42, &Pubkey::default()),
                None,
            ),
        ];
        let instruction_accounts = accounts
            .iter()
            .enumerate()
            .map(|(i, _)| InstructionAccount {
                index_in_transaction: i as u16,
                index_in_caller: i as u16,
                index_in_callee: i as u16,
                is_signer: false,
                is_writable: true,
            })
            .collect::<Vec<_>>();
        let instruction_data = vec![4; 24];
        let slot_context = SlotContext { slot: 42 };
        let epoch_context = EpochContext {
            feature_set: FeatureSet::all_enabled(),
        };

        let metadata = Metadata {
            entrypoint: String::from("Hello, world!"),
        };
        let context = Context {
            program_id,
            accounts,
            instruction_accounts,
            instruction_data,
            compute_units_available: 200_000,
            slot_context,
            epoch_context,
        };
        let effects = Effects::default();

        let fixture = Fixture {
            metadata: Some(metadata),
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
