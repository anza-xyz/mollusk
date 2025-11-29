use {mollusk_svm::Mollusk, solana_account::Account, solana_pubkey::Pubkey};

pub const ID: Pubkey = solana_pubkey::pubkey!("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo");

pub const ELF: &[u8] = include_bytes!("elf/memo-v1.so");

pub fn add_program(mollusk: &mut Mollusk) {
    // Loader v1
    mollusk.add_program_with_loader_and_elf(
        &ID,
        ELF,
        &mollusk_svm::program::loader_keys::LOADER_V1,
    );
}

pub fn account() -> Account {
    // Loader v1
    mollusk_svm::program::create_program_account_loader_v1(ELF)
}

/// Get the key and account for the SPL Memo program V1.
pub fn keyed_account() -> (Pubkey, Account) {
    (ID, account())
}
