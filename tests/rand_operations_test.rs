//! Targeted regression suite for the `rand_core::OsRng` surface this crate uses.
//!
//! Each test mirrors a real call site so a future trait-incompatible bump
//! (ed25519-dalek, k256, rand_core) breaks compilation here first instead of
//! in production code paths.
//!
//! Call sites covered:
//!   - src/crypto.rs::generate_solana_keypair  → SigningKey::generate(&mut OsRng)
//!   - src/crypto.rs::generate_evm_wallet      → EcSecretKey::random(&mut OsRng)
//!   - src/crypto.rs::crypto_es_encrypt        → OsRng.fill_bytes(&mut [0u8; 8])
//!   - src/commands/register.rs (passkey gen)  → OsRng.fill_bytes(&mut [0u8; 32])
//!   - src/commands/register.rs (email gen)    → OsRng.fill_bytes(&mut [0u8; 24])

use ed25519_dalek::SigningKey;
use k256::SecretKey as EcSecretKey;
use rand_core::{OsRng, RngCore};
use skill_trading::crypto::{crypto_es_encrypt, generate_evm_wallet, generate_solana_keypair};

#[test]
fn os_rng_seeds_ed25519_dalek_signing_key() {
    let signing_key = SigningKey::generate(&mut OsRng);
    assert_eq!(signing_key.to_bytes().len(), 32);

    let other = SigningKey::generate(&mut OsRng);
    assert_ne!(
        signing_key.to_bytes(),
        other.to_bytes(),
        "two consecutive ed25519 keys collided — RNG looks deterministic"
    );
}

#[test]
fn os_rng_seeds_k256_secp256k1_secret_key() {
    let secret = EcSecretKey::random(&mut OsRng);
    let bytes = secret.to_bytes();
    assert_eq!(bytes.len(), 32);

    let other = EcSecretKey::random(&mut OsRng);
    assert_ne!(
        bytes.as_slice(),
        other.to_bytes().as_slice(),
        "two consecutive secp256k1 keys collided — RNG looks deterministic"
    );
}

#[test]
fn os_rng_fill_bytes_eight_byte_aes_salt() {
    let mut salt = [0u8; 8];
    OsRng.fill_bytes(&mut salt);
    assert_ne!(salt, [0u8; 8], "8-byte salt was all zeros");

    let mut salt2 = [0u8; 8];
    OsRng.fill_bytes(&mut salt2);
    assert_ne!(salt, salt2, "two consecutive 8-byte salts collided");
}

#[test]
fn os_rng_fill_bytes_thirty_two_byte_passkey() {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    assert_eq!(bytes.len(), 32);
    assert_ne!(bytes, [0u8; 32], "32-byte passkey was all zeros");
    assert!(
        bytes.iter().filter(|b| **b != 0).count() >= 16,
        "passkey entropy looks suspect: too many zero bytes"
    );
}

#[test]
fn os_rng_fill_bytes_twenty_four_byte_email() {
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    assert_eq!(bytes.len(), 24);
    assert_ne!(bytes, [0u8; 24], "24-byte email seed was all zeros");
}

#[test]
fn os_rng_distinct_output_across_calls() {
    let mut a = [0u8; 32];
    let mut b = [0u8; 32];
    OsRng.fill_bytes(&mut a);
    OsRng.fill_bytes(&mut b);
    assert_ne!(a, b, "OsRng emitted identical 32-byte output across calls");
}

#[test]
fn generate_solana_keypair_yields_distinct_keys_per_call() {
    let kp1 = generate_solana_keypair();
    let kp2 = generate_solana_keypair();
    assert_ne!(kp1.public_key, kp2.public_key);
    assert_ne!(kp1.secret_key_hex, kp2.secret_key_hex);
}

#[test]
fn generate_evm_wallet_yields_distinct_keys_per_call() {
    let w1 = generate_evm_wallet();
    let w2 = generate_evm_wallet();
    assert_ne!(w1.private_key, w2.private_key);
    assert_ne!(w1.address, w2.address);

    assert!(w1.address.starts_with("0x") && w1.address.len() == 42);
    assert!(w1.private_key.starts_with("0x") && w1.private_key.len() == 66);
}

#[test]
fn crypto_es_encrypt_uses_random_salt_end_to_end() {
    let a = crypto_es_encrypt("plaintext", "api-key");
    let b = crypto_es_encrypt("plaintext", "api-key");
    assert_ne!(
        a, b,
        "identical inputs produced identical ciphertexts — salt RNG path is broken"
    );
}
