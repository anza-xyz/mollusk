//! Module for working with Solana programs.

use {
    solana_account::Account,
    solana_bpf_loader_program::syscalls::create_program_runtime_environment_v1,
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_feature_set::FeatureSet,
    solana_loader_v3_interface::state::UpgradeableLoaderState,
    solana_program_runtime::{
        invoke_context::BuiltinFunctionWithContext,
        loaded_programs::{LoadProgramMetrics, ProgramCacheEntry, ProgramCacheForTxBatch},
    },
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    std::sync::{Arc, RwLock},
};

/// Loader keys, re-exported from `solana_sdk` for convenience.
pub mod loader_keys {
    pub use solana_sdk_ids::{
        bpf_loader::ID as LOADER_V2, bpf_loader_deprecated::ID as LOADER_V1,
        bpf_loader_upgradeable::ID as LOADER_V3, loader_v4::ID as LOADER_V4,
        native_loader::ID as NATIVE_LOADER,
    };
}

pub mod precompile_keys {
    use solana_pubkey::Pubkey;
    pub use solana_sdk_ids::{
        ed25519_program::ID as ED25519_PROGRAM, secp256k1_program::ID as SECP256K1_PROGRAM,
        secp256r1_program::ID as SECP256R1_PROGRAM,
    };

    pub(crate) fn is_precompile(program_id: &Pubkey) -> bool {
        matches!(
            *program_id,
            ED25519_PROGRAM | SECP256K1_PROGRAM | SECP256R1_PROGRAM
        )
    }
}

pub struct ProgramCache {
    cache: RwLock<ProgramCacheForTxBatch>,
}

impl Default for ProgramCache {
    fn default() -> Self {
        let mut cache = ProgramCacheForTxBatch::default();
        BUILTINS.iter().for_each(|builtin| {
            let program_id = builtin.program_id;
            let entry = builtin.program_cache_entry();
            cache.replenish(program_id, entry);
        });
        Self {
            cache: RwLock::new(cache),
        }
    }
}

impl ProgramCache {
    pub(crate) fn cache(&self) -> &RwLock<ProgramCacheForTxBatch> {
        &self.cache
    }

    /// Add a builtin program to the cache.
    pub fn add_builtin(&mut self, builtin: Builtin) {
        let program_id = builtin.program_id;
        let entry = builtin.program_cache_entry();
        self.cache.write().unwrap().replenish(program_id, entry);
    }

    /// Add a program to the cache.
    pub fn add_program(
        &mut self,
        program_id: &Pubkey,
        loader_key: &Pubkey,
        elf: &[u8],
        compute_budget: &ComputeBudget,
        feature_set: &FeatureSet,
    ) {
        let environment = Arc::new(
            create_program_runtime_environment_v1(feature_set, compute_budget, false, false)
                .unwrap(),
        );
        self.cache.write().unwrap().replenish(
            *program_id,
            Arc::new(
                ProgramCacheEntry::new(
                    loader_key,
                    environment,
                    0,
                    0,
                    elf,
                    elf.len(),
                    &mut LoadProgramMetrics::default(),
                )
                .unwrap(),
            ),
        );
    }

    /// Load a program from the cache.
    pub fn load_program(&self, program_id: &Pubkey) -> Option<Arc<ProgramCacheEntry>> {
        self.cache.read().unwrap().find(program_id)
    }
}

pub struct Builtin {
    program_id: Pubkey,
    name: &'static str,
    entrypoint: BuiltinFunctionWithContext,
}

impl Builtin {
    fn program_cache_entry(&self) -> Arc<ProgramCacheEntry> {
        Arc::new(ProgramCacheEntry::new_builtin(
            0,
            self.name.len(),
            self.entrypoint,
        ))
    }
}

static BUILTINS: &[Builtin] = &[
    Builtin {
        program_id: solana_system_program::id(),
        name: "system_program",
        entrypoint: solana_system_program::system_processor::Entrypoint::vm,
    },
    Builtin {
        program_id: loader_keys::LOADER_V2,
        name: "solana_bpf_loader_program",
        entrypoint: solana_bpf_loader_program::Entrypoint::vm,
    },
    Builtin {
        program_id: loader_keys::LOADER_V3,
        name: "solana_bpf_loader_upgradeable_program",
        entrypoint: solana_bpf_loader_program::Entrypoint::vm,
    },
    #[cfg(feature = "all-builtins")]
    Builtin {
        program_id: solana_sdk_ids::stake::id(),
        name: "solana_stake_program",
        entrypoint: solana_stake_program::stake_instruction::Entrypoint::vm,
    },
    /* ... */
];

/// Create a key and account for a builtin program.
pub fn create_keyed_account_for_builtin_program(
    program_id: &Pubkey,
    name: &str,
) -> (Pubkey, Account) {
    let data = name.as_bytes().to_vec();
    let lamports = Rent::default().minimum_balance(data.len());
    let account = Account {
        lamports,
        data,
        owner: loader_keys::NATIVE_LOADER,
        executable: true,
        ..Default::default()
    };
    (*program_id, account)
}

/// Get the key and account for the system program.
pub fn keyed_account_for_system_program() -> (Pubkey, Account) {
    create_keyed_account_for_builtin_program(&BUILTINS[0].program_id, BUILTINS[0].name)
}

/// Get the key and account for the BPF Loader v2 program.
pub fn keyed_account_for_bpf_loader_v2_program() -> (Pubkey, Account) {
    create_keyed_account_for_builtin_program(&BUILTINS[1].program_id, BUILTINS[1].name)
}

/// Get the key and account for the BPF Loader v3 (Upgradeable) program.
pub fn keyed_account_for_bpf_loader_v3_program() -> (Pubkey, Account) {
    create_keyed_account_for_builtin_program(&BUILTINS[1].program_id, BUILTINS[1].name)
}

/* ... */

/// Create a BPF Loader 1 (deprecated) program account.
pub fn create_program_account_loader_v1(elf: &[u8]) -> Account {
    let lamports = Rent::default().minimum_balance(elf.len());
    Account {
        lamports,
        data: elf.to_vec(),
        owner: loader_keys::LOADER_V1,
        executable: true,
        ..Default::default()
    }
}

/// Create a BPF Loader 2 program account.
pub fn create_program_account_loader_v2(elf: &[u8]) -> Account {
    let lamports = Rent::default().minimum_balance(elf.len());
    Account {
        lamports,
        data: elf.to_vec(),
        owner: loader_keys::LOADER_V2,
        executable: true,
        ..Default::default()
    }
}

/// Create a BPF Loader v3 (Upgradeable) program account.
pub fn create_program_account_loader_v3(program_id: &Pubkey) -> Account {
    let programdata_address =
        Pubkey::find_program_address(&[program_id.as_ref()], &loader_keys::LOADER_V3).0;
    let data = bincode::serialize(&UpgradeableLoaderState::Program {
        programdata_address,
    })
    .unwrap();
    let lamports = Rent::default().minimum_balance(data.len());
    Account {
        lamports,
        data,
        owner: loader_keys::LOADER_V3,
        executable: true,
        ..Default::default()
    }
}

/// Create a BPF Loader v3 (Upgradeable) program data account.
pub fn create_program_data_account_loader_v3(elf: &[u8]) -> Account {
    let data = {
        let elf_offset = UpgradeableLoaderState::size_of_programdata_metadata();
        let data_len = elf_offset + elf.len();
        let mut data = vec![0; data_len];
        bincode::serialize_into(
            &mut data[0..elf_offset],
            &UpgradeableLoaderState::ProgramData {
                slot: 0,
                upgrade_authority_address: None,
            },
        )
        .unwrap();
        data[elf_offset..].copy_from_slice(elf);
        data
    };
    let lamports = Rent::default().minimum_balance(data.len());
    Account {
        lamports,
        data,
        owner: loader_keys::LOADER_V3,
        executable: false,
        ..Default::default()
    }
}

/// Create a BPF Loader v3 (Upgradeable) program and program data account.
///
/// Returns a tuple, where the first element is the program account and the
/// second element is the program data account.
pub fn create_program_account_pair_loader_v3(
    program_id: &Pubkey,
    elf: &[u8],
) -> (Account, Account) {
    (
        create_program_account_loader_v3(program_id),
        create_program_data_account_loader_v3(elf),
    )
}
