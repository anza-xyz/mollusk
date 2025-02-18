use {
    prost::Message,
    serde::{de::DeserializeOwned, Serialize},
    std::{
        fs::{self, File},
        io::{Read, Write},
        path::Path,
    },
};

/// Represents a serializable fuzz fixture.
pub trait SerializableFixture: Default + DeserializeOwned + Message + Serialize + Sized {
    /// Decode a `Protobuf` blob into a fixture.
    fn decode(blob: &[u8]) -> Self {
        <Self as Message>::decode(blob)
            .unwrap_or_else(|err| panic!("Failed to decode fixture: {}", err))
    }

    /// Encode the fixture into a `Protobuf` blob.
    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        Message::encode(self, &mut buf).expect("Failed to encode fixture");
        buf
    }

    /// Hash the fixture's contents into a Keccak hash.
    fn hash(&self) -> solana_keccak_hasher::Hash;
}

/// Represents a fixture that can be converted into a serializable fixture.
pub trait IntoSerializableFixture {
    type Fixture: SerializableFixture;

    fn into(self) -> Self::Fixture;
}

pub struct FsHandler<SF>
where
    SF: SerializableFixture,
{
    serializable_fixture: SF,
}

impl<SF> FsHandler<SF>
where
    SF: SerializableFixture,
{
    pub fn new<T: IntoSerializableFixture<Fixture = SF>>(fix: T) -> Self {
        let serializable_fixture = fix.into();
        Self {
            serializable_fixture,
        }
    }

    /// Dumps the fixture to a protobuf binary blob file.
    /// The file name is a hash of the fixture with the `.fix` extension.
    pub fn dump_to_blob_file(&self, dir: &str) {
        let blob = SerializableFixture::encode(&self.serializable_fixture);

        let hash = self.serializable_fixture.hash();
        let file_name = format!("instr-{}.fix", bs58::encode(hash).into_string());

        write_file(Path::new(dir), &file_name, &blob);
    }

    /// Dumps the fixture to a JSON file.
    /// The file name is a hash of the fixture with the `.json` extension.
    pub fn dump_to_json_file(self, dir_path: &str) {
        let json = serde_json::to_string_pretty(&self.serializable_fixture)
            .expect("Failed to serialize fixture to JSON");

        let hash = self.serializable_fixture.hash();
        let file_name = format!("instr-{}.json", bs58::encode(hash).into_string());

        write_file(Path::new(dir_path), &file_name, json.as_bytes());
    }

    /// Loads a fixture from a protobuf binary blob file.
    pub fn load_from_blob_file(file_path: &str) -> SF {
        if !file_path.ends_with(".fix") {
            panic!("Invalid fixture file extension: {}", file_path);
        }
        let mut file = File::open(file_path).expect("Failed to open fixture file");
        let mut blob = Vec::new();
        file.read_to_end(&mut blob)
            .expect("Failed to read fixture file");
        <SF as SerializableFixture>::decode(&blob)
    }

    /// Loads a fixture from a JSON file.
    pub fn load_from_json_file(file_path: &str) -> SF {
        if !file_path.ends_with(".json") {
            panic!("Invalid fixture file extension: {}", file_path);
        }
        let mut file = File::open(file_path).expect("Failed to open fixture file");
        let mut json = String::new();
        file.read_to_string(&mut json)
            .expect("Failed to read fixture file");
        serde_json::from_str(&json).expect("Failed to deserialize fixture from JSON")
    }
}

fn write_file(dir: &Path, file_name: &str, data: &[u8]) {
    fs::create_dir_all(dir).expect("Failed to create directory");
    let file_path = dir.join(file_name);
    let mut file = File::create(file_path).unwrap();
    file.write_all(data)
        .expect("Failed to write fixture to file");
}
