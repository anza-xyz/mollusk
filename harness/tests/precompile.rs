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

    let msg = b"hello";
    let sk_bytes = secret_key.serialize();
    let (sig, recid) = solana_secp256k1_program::sign_message(&sk_bytes, msg).unwrap();
    let pubkey = libsecp256k1::PublicKey::from_secret_key(&secret_key);
    let uncompressed = pubkey.serialize(); // 65 bytes, 0x04 || X(32) || Y(32)
    let mut uncompressed_64 = [0u8; 64];
    uncompressed_64.copy_from_slice(&uncompressed[1..65]);
    let eth_address = solana_secp256k1_program::eth_address_from_pubkey(&uncompressed_64);
    let instr = solana_secp256k1_program::new_secp256k1_instruction_with_signature(
        msg,
        &sig,
        recid,
        &eth_address,
    );

    mollusk.process_and_validate_instruction(
        &instr,
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

#[test]
fn test_ed25519() {
    use ed25519_dalek::Signer;
    let mollusk = Mollusk::default();
    let keypair = ed25519_dalek::Keypair::generate(&mut thread_rng());

    let msg = b"hello";
    let signature = keypair.sign(msg).to_bytes();
    let pubkey_bytes = keypair.public.to_bytes();

    let instr = solana_ed25519_program::new_ed25519_instruction_with_signature(
        msg,
        <&[u8; solana_ed25519_program::SIGNATURE_SERIALIZED_SIZE]>::try_from(&signature[..])
            .unwrap(),
        <&[u8; solana_ed25519_program::PUBKEY_SERIALIZED_SIZE]>::try_from(&pubkey_bytes[..])
            .unwrap(),
    );

    mollusk.process_and_validate_instruction(
        &instr,
        &[
            (Pubkey::new_unique(), Account::default()),
            (solana_sdk_ids::ed25519_program::id(), precompile_account()),
        ],
        &[Check::success()],
    );
}

#[test]
fn test_secp256r1() {
    use openssl::{
        bn::BigNumContext,
        ec::{EcGroup, EcKey, PointConversionForm},
        nid::Nid,
    };

    let mollusk = Mollusk::default();
    let secret_key = {
        let curve_name = Nid::X9_62_PRIME256V1;
        let group = EcGroup::from_curve_name(curve_name).unwrap();
        EcKey::generate(&group).unwrap()
    };

    let sig =
        solana_secp256r1_program::sign_message(b"hello", &secret_key.private_key_to_der().unwrap())
            .unwrap();
    let mut ctx = BigNumContext::new().unwrap();
    let pub_bytes = secret_key
        .public_key()
        .to_bytes(
            secret_key.group(),
            PointConversionForm::COMPRESSED,
            &mut ctx,
        )
        .unwrap();
    let mut pubkey = [0u8; solana_secp256r1_program::COMPRESSED_PUBKEY_SERIALIZED_SIZE];
    pubkey.copy_from_slice(&pub_bytes);

    let instr =
        solana_secp256r1_program::new_secp256r1_instruction_with_signature(b"hello", &sig, &pubkey);

    mollusk.process_and_validate_instruction(
        &instr,
        &[
            (Pubkey::new_unique(), Account::default()),
            (solana_sdk_ids::ed25519_program::id(), precompile_account()),
        ],
        &[Check::success()],
    );
}
