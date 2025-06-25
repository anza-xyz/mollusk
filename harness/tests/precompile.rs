use {
    mollusk_svm::{result::Check, Mollusk},
    rand0_7::thread_rng,
    solana_account::{Account, WritableAccount},
    solana_pubkey::Pubkey,
};

fn precompile_account() -> Account {
    let mut account = Account::new(1, 0, &solana_sdk_ids::native_loader::id());
    account.set_executable(true);
    account
}

#[test]
fn test_secp256k1() {
    let mollusk = Mollusk::default();
    let secret_key = libsecp256k1::SecretKey::random(&mut thread_rng());

    mollusk.process_and_validate_instruction(
        &solana_secp256k1_program::new_secp256k1_instruction(&secret_key, b"hello"),
        &[
            (Pubkey::new_unique(), Account::default()),
            (
                solana_sdk_ids::secp256k1_program::id(),
                precompile_account(),
            ),
        ],
        &[Check::success()],
    );
}

#[allow(deprecated)]
#[test]
fn test_ed25519() {
    let mollusk = Mollusk::default();
    let secret_key = ed25519_dalek::Keypair::generate(&mut thread_rng());

    mollusk.process_and_validate_instruction(
        &solana_ed25519_program::new_ed25519_instruction(&secret_key, b"hello"),
        &[
            (Pubkey::new_unique(), Account::default()),
            (solana_sdk_ids::ed25519_program::id(), precompile_account()),
        ],
        &[Check::success()],
    );
}

#[allow(deprecated)]
#[test]
fn test_secp256r1() {
    use openssl::{
        ec::{EcGroup, EcKey},
        nid::Nid,
    };

    let mollusk = Mollusk::default();
    let secret_key = {
        let curve_name = Nid::X9_62_PRIME256V1;
        let group = EcGroup::from_curve_name(curve_name).unwrap();
        EcKey::generate(&group).unwrap()
    };

    mollusk.process_and_validate_instruction(
        &solana_secp256r1_program::new_secp256r1_instruction(b"hello", secret_key).unwrap(),
        &[
            (Pubkey::new_unique(), Account::default()),
            (solana_sdk_ids::ed25519_program::id(), precompile_account()),
        ],
        &[Check::success()],
    );
}
