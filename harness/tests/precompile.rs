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
    let priv_bytes: [u8; 32] = secret_key.serialize();
    let (sig64, rec_id) = solana_secp256k1_program::sign_message(&priv_bytes, msg).unwrap();
    let pubkey = libsecp256k1::PublicKey::from_secret_key(&secret_key);
    let pubkey_uncompressed = pubkey.serialize(); // 65 bytes: 0x04 || X || Y
    let mut xy = [0u8; 64];
    xy.copy_from_slice(&pubkey_uncompressed[1..65]);
    let eth_addr = solana_secp256k1_program::eth_address_from_pubkey(&xy);

    mollusk.process_and_validate_instruction(
        &solana_secp256k1_program::new_secp256k1_instruction_with_signature(
            msg, &sig64, rec_id, &eth_addr,
        ),
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
    let mollusk = Mollusk::default();
    use ed25519_dalek::Signer;
    let secret_key = ed25519_dalek::Keypair::generate(&mut thread_rng());

    let msg = b"hello";
    let signature = secret_key.sign(msg);
    let pubkey = secret_key.public;

    mollusk.process_and_validate_instruction(
        &solana_ed25519_program::new_ed25519_instruction_with_signature(
            msg,
            &signature.to_bytes(),
            &pubkey.to_bytes(),
        ),
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
        ecdsa::EcdsaSig,
        hash::MessageDigest,
        nid::Nid,
        pkey::PKey,
    };

    let mollusk = Mollusk::default();
    let secret_key = {
        let curve_name = Nid::X9_62_PRIME256V1;
        let group = EcGroup::from_curve_name(curve_name).unwrap();
        EcKey::generate(&group).unwrap()
    };

    let msg = b"hello";
    // Sign SHA-256(message) and get DER
    let pkey = PKey::from_ec_key(secret_key.clone()).unwrap();
    let mut signer = openssl::sign::Signer::new(MessageDigest::sha256(), &pkey).unwrap();
    signer.update(msg).unwrap();
    let der = signer.sign_to_vec().unwrap();
    // Convert DER to raw (r||s) 64 bytes with low-S canonicalization
    let sig = EcdsaSig::from_der(&der).unwrap();
    let mut ctx = BigNumContext::new().unwrap();
    let mut order = openssl::bn::BigNum::new().unwrap();
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    group.order(&mut order, &mut ctx).unwrap();
    let mut order_minus_s = openssl::bn::BigNum::new().unwrap();
    order_minus_s.checked_sub(&order, sig.s()).unwrap();
    let s_ref = if sig.s().ucmp(&order_minus_s) == std::cmp::Ordering::Greater {
        &order_minus_s
    } else {
        sig.s()
    };
    let mut r = sig.r().to_vec();
    let mut s = s_ref.to_vec();
    if r.len() < 32 {
        r.splice(0..0, std::iter::repeat(0u8).take(32 - r.len()));
    }
    if s.len() < 32 {
        s.splice(0..0, std::iter::repeat(0u8).take(32 - s.len()));
    }
    let mut raw64 = [0u8; 64];
    raw64[..32].copy_from_slice(&r[..32]);
    raw64[32..].copy_from_slice(&s[..32]);
    // Compressed pubkey (33 bytes)
    let pubkey_point = secret_key.public_key();
    let compressed = pubkey_point
        .to_bytes(&group, PointConversionForm::COMPRESSED, &mut ctx)
        .unwrap();
    let mut comp33 = [0u8; 33];
    comp33.copy_from_slice(&compressed);
    mollusk.process_and_validate_instruction(
        &solana_secp256r1_program::new_secp256r1_instruction_with_signature(msg, &raw64, &comp33),
        &[
            (Pubkey::new_unique(), Account::default()),
            (
                solana_sdk_ids::secp256r1_program::id(),
                precompile_account(),
            ),
        ],
        &[Check::success()],
    );
}
