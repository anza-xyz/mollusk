//! Program invocation metadata.

use {super::proto::FixtureMetadata as ProtoFixtureMetadata, solana_keccak_hasher::Hasher};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Metadata {
    /// The program entrypoint function name.
    pub entrypoint: String,
}

impl From<ProtoFixtureMetadata> for Metadata {
    fn from(value: ProtoFixtureMetadata) -> Self {
        Self {
            entrypoint: value.fn_entrypoint,
        }
    }
}

impl From<Metadata> for ProtoFixtureMetadata {
    fn from(value: Metadata) -> Self {
        Self {
            fn_entrypoint: value.entrypoint,
        }
    }
}

pub(crate) fn hash_proto_metadata(hasher: &mut Hasher, metadata: &ProtoFixtureMetadata) {
    hasher.hash(metadata.fn_entrypoint.as_bytes());
}
