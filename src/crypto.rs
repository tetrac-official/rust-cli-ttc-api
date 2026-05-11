//! Cryptographic utilities for wallet generation and encryption.
//!
//! Implements algorithms that match the Tetrac web client:
//! - PBKDF2-SHA1 apiKey derivation (deriveApiKeyFromPasskey)
//! - SHA-256 passkey hashing (hashPasskeyForServer)
//! - OpenSSL EVP AES-256-CBC encryption (CryptoES.AES.encrypt)
//! - Ed25519 keypair generation (Solana / Orderly format)
//! - secp256k1 wallet generation (EVM / Ethereum format)

use aes::Aes256;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use cbc::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
use ed25519_dalek::SigningKey;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::SecretKey as EcSecretKey;
use md5::Md5;
use pbkdf2::pbkdf2_hmac;
use rand_core::{OsRng, RngCore};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use sha3::Keccak256;

type Aes256CbcEnc = cbc::Encryptor<Aes256>;

// ============================================================================
// Wallet Generation
// ============================================================================

pub struct SolanaKeypair {
    /// Base58-encoded 32-byte public key (Solana address format)
    pub public_key: String,
    /// Hex-encoded 64-byte keypair: secret_scalar || public_key
    pub secret_key_hex: String,
}

pub struct EvmWallet {
    /// Ethereum address: "0x" + 40 lowercase hex chars
    pub address: String,
    /// Private key: "0x" + 64 lowercase hex chars
    pub private_key: String,
}

/// Generate a new Ed25519 keypair compatible with Solana/Orderly wallet format.
pub fn generate_solana_keypair() -> SolanaKeypair {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let pub_bytes = verifying_key.to_bytes();

    let public_key = bs58::encode(&pub_bytes).into_string();

    // Solana secretKey format: 64 bytes = seed (32) || public_key (32)
    let mut full_kp = [0u8; 64];
    full_kp[..32].copy_from_slice(&signing_key.to_bytes());
    full_kp[32..].copy_from_slice(&pub_bytes);
    let secret_key_hex = hex::encode(full_kp);

    SolanaKeypair {
        public_key,
        secret_key_hex,
    }
}

/// Generate a new secp256k1 private key and derive its Ethereum address.
pub fn generate_evm_wallet() -> EvmWallet {
    let secret = EcSecretKey::random(&mut OsRng);
    let pubkey = secret.public_key();
    let point = pubkey.to_encoded_point(false); // uncompressed: 0x04 || x(32) || y(32)
    let pub_bytes = &point.as_bytes()[1..]; // drop 0x04 prefix → 64 bytes

    let hash = Keccak256::digest(pub_bytes);
    let address = format!("0x{}", hex::encode(&hash[12..])); // last 20 bytes
    let private_key = format!("0x{}", hex::encode(secret.to_bytes()));

    EvmWallet {
        address,
        private_key,
    }
}

// ============================================================================
// Key Derivation & Hashing
// ============================================================================

/// Derive the client-side apiKey from passkey + email.
/// Matches `deriveApiKeyFromPasskey` in authUtils.ts:
///   PBKDF2-SHA1(passkey, email.toLowerCase().trim(), 100_000 iterations, 256 bits)
/// Returns a 64-char lowercase hex string.
pub fn derive_api_key(passkey: &str, email: &str) -> String {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha1>(
        passkey.as_bytes(),
        email.to_lowercase().trim().as_bytes(),
        100_000,
        &mut key,
    );
    hex::encode(key)
}

/// Hash the passkey for server-side storage.
/// Matches `hashPasskeyForServer` in authUtils.ts: SHA-256(passkey as UTF-8)
/// Returns a 64-char lowercase hex string.
pub fn hash_passkey_for_server(passkey: &str) -> String {
    hex::encode(Sha256::digest(passkey.as_bytes()))
}

// ============================================================================
// CryptoES-Compatible Encryption
// ============================================================================

/// Encrypt `plaintext` using the OpenSSL EVP AES-256-CBC format.
/// Matches `CryptoES.AES.encrypt(plaintext, apiKey).toString()`.
///
/// Output format (base64-encoded):
///   "Salted__" (8 bytes) || salt (8 bytes) || AES-256-CBC(key, iv, PKCS7(plaintext))
///
/// Key + IV derived via EVP_BytesToKey (MD5, 1 iteration):
///   D1 = MD5(password || salt)
///   D2 = MD5(D1 || password || salt)
///   key = D1 || D2   (32 bytes)
///   iv  = MD5(D2 || password || salt)  (16 bytes)
pub fn crypto_es_encrypt(plaintext: &str, api_key: &str) -> String {
    let mut salt = [0u8; 8];
    OsRng.fill_bytes(&mut salt);

    let (key, iv) = evp_bytes_to_key(api_key.as_bytes(), &salt);

    let ciphertext = Aes256CbcEnc::new(&key.into(), &iv.into())
        .encrypt_padded_vec_mut::<Pkcs7>(plaintext.as_bytes());

    let mut out = Vec::with_capacity(16 + ciphertext.len());
    out.extend_from_slice(b"Salted__");
    out.extend_from_slice(&salt);
    out.extend_from_slice(&ciphertext);

    BASE64.encode(&out)
}

/// OpenSSL EVP_BytesToKey with MD5, 1 iteration.
/// Produces 32-byte key and 16-byte IV for AES-256-CBC.
fn evp_bytes_to_key(password: &[u8], salt: &[u8]) -> ([u8; 32], [u8; 16]) {
    let d1 = {
        let mut h = Md5::new();
        h.update(password);
        h.update(salt);
        h.finalize()
    };
    let d2 = {
        let mut h = Md5::new();
        h.update(d1);
        h.update(password);
        h.update(salt);
        h.finalize()
    };
    let d3 = {
        let mut h = Md5::new();
        h.update(d2);
        h.update(password);
        h.update(salt);
        h.finalize()
    };

    let mut key = [0u8; 32];
    let mut iv = [0u8; 16];
    key[..16].copy_from_slice(&d1);
    key[16..].copy_from_slice(&d2);
    iv.copy_from_slice(&d3);
    (key, iv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut};
    use ed25519_dalek::{Signer, Verifier};

    type Aes256CbcDec = cbc::Decryptor<Aes256>;

    fn is_lowercase_hex(s: &str) -> bool {
        s.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    }

    fn decrypt_crypto_es(ciphertext_b64: &str, api_key: &str) -> Option<String> {
        let raw = BASE64.decode(ciphertext_b64).ok()?;
        if raw.len() < 16 || &raw[..8] != b"Salted__" {
            return None;
        }
        let salt: [u8; 8] = raw[8..16].try_into().ok()?;
        let body = &raw[16..];
        let (key, iv) = evp_bytes_to_key(api_key.as_bytes(), &salt);
        let mut buf = body.to_vec();
        let pt = Aes256CbcDec::new(&key.into(), &iv.into())
            .decrypt_padded_mut::<Pkcs7>(&mut buf)
            .ok()?;
        String::from_utf8(pt.to_vec()).ok()
    }

    // ---- derive_api_key ----------------------------------------------------

    #[test]
    fn derive_api_key_is_deterministic() {
        let a = derive_api_key("hunter2", "alice@example.com");
        let b = derive_api_key("hunter2", "alice@example.com");
        assert_eq!(a, b);
    }

    #[test]
    fn derive_api_key_normalizes_email() {
        let canonical = derive_api_key("pk", "alice@example.com");
        assert_eq!(derive_api_key("pk", "ALICE@EXAMPLE.COM"), canonical);
        assert_eq!(derive_api_key("pk", "  Alice@Example.com  "), canonical);
    }

    #[test]
    fn derive_api_key_returns_64_lowercase_hex() {
        let k = derive_api_key("pk", "a@b.com");
        assert_eq!(k.len(), 64);
        assert!(is_lowercase_hex(&k), "not lowercase hex: {k}");
    }

    #[test]
    fn derive_api_key_different_passkeys_differ() {
        let a = derive_api_key("pk-one", "a@b.com");
        let b = derive_api_key("pk-two", "a@b.com");
        assert_ne!(a, b);
    }

    #[test]
    fn derive_api_key_different_emails_differ() {
        let a = derive_api_key("pk", "a@b.com");
        let b = derive_api_key("pk", "c@d.com");
        assert_ne!(a, b);
    }

    // ---- hash_passkey_for_server ------------------------------------------

    #[test]
    fn hash_passkey_for_server_matches_known_vector() {
        // SHA-256("abc") — RFC 6234 / FIPS 180-4 test vector
        assert_eq!(
            hash_passkey_for_server("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hash_passkey_for_server_returns_64_lowercase_hex() {
        let h = hash_passkey_for_server("anything");
        assert_eq!(h.len(), 64);
        assert!(is_lowercase_hex(&h));
    }

    #[test]
    fn hash_passkey_for_server_is_deterministic() {
        assert_eq!(
            hash_passkey_for_server("same"),
            hash_passkey_for_server("same")
        );
    }

    // ---- generate_solana_keypair ------------------------------------------

    #[test]
    fn solana_keypair_has_correct_sizes() {
        let kp = generate_solana_keypair();
        // secret_key_hex = 64 bytes = 128 hex chars
        assert_eq!(kp.secret_key_hex.len(), 128);
        assert!(is_lowercase_hex(&kp.secret_key_hex));
        // public_key is base58 of 32 bytes
        let pub_bytes = bs58::decode(&kp.public_key).into_vec().expect("base58");
        assert_eq!(pub_bytes.len(), 32);
    }

    #[test]
    fn solana_secret_last_32_bytes_match_public_key() {
        let kp = generate_solana_keypair();
        let secret_bytes = hex::decode(&kp.secret_key_hex).unwrap();
        let pub_from_secret = &secret_bytes[32..];
        let pub_decoded = bs58::decode(&kp.public_key).into_vec().unwrap();
        assert_eq!(pub_from_secret, pub_decoded.as_slice());
    }

    #[test]
    fn solana_keypair_is_unique_per_call() {
        let a = generate_solana_keypair();
        let b = generate_solana_keypair();
        assert_ne!(a.public_key, b.public_key);
        assert_ne!(a.secret_key_hex, b.secret_key_hex);
    }

    #[test]
    fn solana_keypair_signs_and_verifies() {
        let kp = generate_solana_keypair();
        let secret_bytes = hex::decode(&kp.secret_key_hex).unwrap();
        let seed: [u8; 32] = secret_bytes[..32].try_into().unwrap();
        let signing = SigningKey::from_bytes(&seed);
        let verifying = signing.verifying_key();

        // Verifying key derived from seed must match the announced public key
        let pub_decoded = bs58::decode(&kp.public_key).into_vec().unwrap();
        assert_eq!(verifying.to_bytes().as_slice(), pub_decoded.as_slice());

        let msg = b"hello ttc";
        let sig = signing.sign(msg);
        verifying.verify(msg, &sig).expect("signature must verify");
    }

    // ---- generate_evm_wallet ----------------------------------------------

    #[test]
    fn evm_wallet_has_correct_format() {
        let w = generate_evm_wallet();
        assert!(w.address.starts_with("0x"));
        assert_eq!(w.address.len(), 42); // "0x" + 40 hex
        assert!(is_lowercase_hex(&w.address[2..]));

        assert!(w.private_key.starts_with("0x"));
        assert_eq!(w.private_key.len(), 66); // "0x" + 64 hex
        assert!(is_lowercase_hex(&w.private_key[2..]));
    }

    #[test]
    fn evm_wallet_is_unique_per_call() {
        let a = generate_evm_wallet();
        let b = generate_evm_wallet();
        assert_ne!(a.address, b.address);
        assert_ne!(a.private_key, b.private_key);
    }

    #[test]
    fn evm_address_is_derivable_from_private_key() {
        let w = generate_evm_wallet();
        let pk_bytes = hex::decode(&w.private_key[2..]).unwrap();
        let secret = EcSecretKey::from_slice(&pk_bytes).unwrap();
        let pubkey = secret.public_key();
        let point = pubkey.to_encoded_point(false);
        let pub_bytes = &point.as_bytes()[1..];
        let hash = Keccak256::digest(pub_bytes);
        let expected = format!("0x{}", hex::encode(&hash[12..]));
        assert_eq!(w.address, expected);
    }

    // ---- crypto_es_encrypt -------------------------------------------------

    #[test]
    fn crypto_es_encrypt_emits_salted_prefix() {
        let ct = crypto_es_encrypt("payload", "any-key");
        let raw = BASE64.decode(&ct).expect("valid base64");
        assert!(raw.len() >= 16, "too short: {}", raw.len());
        assert_eq!(&raw[..8], b"Salted__");
        // body must be a positive multiple of 16 (AES block size, PKCS7-padded)
        let body_len = raw.len() - 16;
        assert!(body_len > 0 && body_len.is_multiple_of(16));
    }

    #[test]
    fn crypto_es_encrypt_uses_random_salt() {
        let a = crypto_es_encrypt("same plaintext", "same-key");
        let b = crypto_es_encrypt("same plaintext", "same-key");
        assert_ne!(a, b, "salt should be random");
    }

    #[test]
    fn crypto_es_encrypt_round_trips() {
        let plaintext = "private-key-payload-12345";
        let key = "derived-api-key";
        let ct = crypto_es_encrypt(plaintext, key);
        let recovered = decrypt_crypto_es(&ct, key).expect("decrypt");
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn crypto_es_encrypt_round_trips_long_payload() {
        // Cross block boundaries to confirm PKCS7 padding round-trips.
        let plaintext: String = "a".repeat(100);
        let ct = crypto_es_encrypt(&plaintext, "k");
        let recovered = decrypt_crypto_es(&ct, "k").expect("decrypt");
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn crypto_es_encrypt_round_trips_empty_payload() {
        let ct = crypto_es_encrypt("", "k");
        let recovered = decrypt_crypto_es(&ct, "k").expect("decrypt empty");
        assert_eq!(recovered, "");
    }

    #[test]
    fn crypto_es_encrypt_wrong_key_does_not_recover_plaintext() {
        let plaintext = "secret";
        let ct = crypto_es_encrypt(plaintext, "right-key");
        // PKCS7 unpadding usually rejects a wrong key, but on the rare chance
        // it produces valid-looking padding, the bytes still won't equal the
        // original plaintext. Both outcomes are acceptable; what's NOT
        // acceptable is silently recovering the original.
        match decrypt_crypto_es(&ct, "wrong-key") {
            None => {}
            Some(s) => assert_ne!(s, plaintext, "wrong key must not recover plaintext"),
        }
    }
}
