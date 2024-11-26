use {
    mollusk_svm::Mollusk,
    solana_sdk::{account::AccountSharedData, pubkey::Pubkey},
};

pub const ID: Pubkey = solana_sdk::pubkey!("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo");

pub const ELF: &[u8] = include_bytes!("elf/memo-v1.so");

pub fn add_program(mollusk: &mut Mollusk) {
    // Loader v1
    mollusk.add_program_with_elf_and_loader(
        &ID,
        ELF,
        &mollusk_svm::program::loader_keys::LOADER_V1,
    );
}

pub fn account() -> AccountSharedData {
    // Loader v1
    mollusk_svm::program::create_program_account_loader_v1(ELF)
}

/// Get the key and account for the SPL Memo program V1.
pub fn keyed_account() -> (Pubkey, AccountSharedData) {
    (ID, account())
}
