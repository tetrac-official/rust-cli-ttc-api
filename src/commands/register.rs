//! Register command — create a new TTC Box account.
//!
//! Auto-generates a random passkey (SHA-256 strength, 64-char hex).
//! Derives wallet keys, encrypts them client-side, POSTs to /api/auth/register.
//! Saves TTC_EMAIL, TTC_PASSKEY, TTC_AUTH_TOKEN, TTC_PUBLIC_KEY,
//! and TTC_TOKEN_ISSUED_AT to .env. Never overwrites exchange API keys.

use crate::cli::RegisterArgs;
use crate::config::AppConfig;
use crate::crypto::{
    crypto_es_encrypt, derive_api_key, generate_evm_wallet, generate_solana_keypair,
    hash_passkey_for_server,
};
use crate::error::{Result, TtcError};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// Request / Response types
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterRequest {
    email: String,
    hashed_passkey: String,
    client_generated_wallets: ClientWallets,
}

#[derive(Debug, Serialize)]
struct ClientWallets {
    solana: SolanaWallet,
    orderly: SolanaWallet,
    evm: EvmWalletPayload,
    #[serde(rename = "evmSigning")]
    evm_signing: EvmWalletPayload,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SolanaWallet {
    public_key: String,
    encrypted_private_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvmWalletPayload {
    address: String,
    encrypted_private_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterResponse {
    success: bool,
    auth_token: Option<String>,
    message: Option<String>,
    error: Option<String>,
    #[serde(default)]
    is_new_user: Option<bool>,
}

// ============================================================================
// Command implementation
// ============================================================================

pub async fn execute(args: RegisterArgs, settings: &AppConfig) -> Result<()> {
    // Email: use arg/env, or auto-generate
    let email = match args.email {
        Some(e) if !e.is_empty() => e,
        _ => generate_email(),
    };

    // Passkey: reuse TTC_PASSKEY from env if already set, otherwise generate new
    let passkey = match std::env::var("TTC_PASSKEY") {
        Ok(p) if !p.is_empty() => {
            println!("Using existing passkey from TTC_PASSKEY.");
            p
        }
        _ => {
            let mut bytes = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut bytes);
            hex::encode(bytes)
        }
    };

    println!("Email:   {}", email);
    println!("Generating wallets and encrypting keys...");

    let api_key = derive_api_key(&passkey, &email);
    let hashed_passkey = hash_passkey_for_server(&passkey);

    // Generate wallets
    let solana_kp = generate_solana_keypair();
    let orderly_kp = generate_solana_keypair();
    let evm_main = generate_evm_wallet();
    let evm_signing_wallet = generate_evm_wallet();

    let client_wallets = ClientWallets {
        solana: SolanaWallet {
            public_key: solana_kp.public_key.clone(),
            encrypted_private_key: crypto_es_encrypt(&solana_kp.secret_key_hex, &api_key),
        },
        orderly: SolanaWallet {
            public_key: orderly_kp.public_key,
            encrypted_private_key: crypto_es_encrypt(&orderly_kp.secret_key_hex, &api_key),
        },
        evm: EvmWalletPayload {
            address: evm_main.address.clone(),
            encrypted_private_key: crypto_es_encrypt(&evm_main.private_key, &api_key),
        },
        evm_signing: EvmWalletPayload {
            address: evm_signing_wallet.address,
            encrypted_private_key: crypto_es_encrypt(&evm_signing_wallet.private_key, &api_key),
        },
    };

    let body = RegisterRequest {
        email: email.clone(),
        hashed_passkey,
        client_generated_wallets: client_wallets,
    };

    // POST to /api/auth/register
    let base = settings.api.base_url.trim_end_matches('/');
    let register_url = if let Some(pos) = base.find("/api/") {
        format!("{}/api/auth/register", &base[..pos])
    } else {
        "https://ttc.box/api/auth/register".to_string()
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(settings.api.timeout))
        .build()
        .map_err(|e| TtcError::Config(format!("Failed to build HTTP client: {}", e)))?;

    let resp = client
        .post(&register_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| TtcError::Config(format!("Register request failed: {}", e)))?;

    let status = resp.status().as_u16();
    let reg_resp: RegisterResponse = resp
        .json()
        .await
        .map_err(|e| TtcError::Config(format!("Failed to parse register response: {}", e)))?;

    if !reg_resp.success || reg_resp.auth_token.is_none() {
        let msg = reg_resp
            .error
            .or(reg_resp.message)
            .unwrap_or_else(|| format!("Registration failed (HTTP {})", status));
        return Err(TtcError::Api { code: status, message: msg });
    }

    let auth_token = reg_resp.auth_token.unwrap();
    let is_new = reg_resp.is_new_user.unwrap_or(true);

    update_env_file(&email, &passkey, &auth_token, &solana_kp.public_key)?;

    if is_new {
        println!("Registration successful.");
    } else {
        println!("Account already exists — session refreshed.");
    }
    println!("Saved to .env: TTC_EMAIL, TTC_PASSKEY, TTC_AUTH_TOKEN, TTC_PUBLIC_KEY, TTC_TOKEN_ISSUED_AT");
    println!("Public key:  {}", solana_kp.public_key);
    println!("Token expires in 24 hours — run `skill-trading login` to refresh.");

    Ok(())
}

/// Generate a random email in the format <base64url>@d<days>.box
fn generate_email() -> String {
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    let random_part = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        &bytes,
    );
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() / 86400;
    format!("{}@d{}.box", random_part, days)
}

/// Update .env with TTC session variables only.
/// Explicitly allowlists what it will touch — all other lines are preserved.
pub fn update_env_file(email: &str, passkey: &str, auth_token: &str, public_key: &str) -> Result<()> {
    let env_path = PathBuf::from(".env");

    let existing = if env_path.exists() {
        std::fs::read_to_string(&env_path)
            .map_err(|e| TtcError::Config(format!("Failed to read .env: {}", e)))?
    } else {
        String::new()
    };

    let issued_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut lines: Vec<String> = existing.lines().map(|l| l.to_string()).collect();
    let mut found: HashMap<&str, bool> = [
        ("TTC_EMAIL", false),
        ("TTC_PASSKEY", false),
        ("TTC_AUTH_TOKEN", false),
        ("TTC_PUBLIC_KEY", false),
        ("TTC_TOKEN_ISSUED_AT", false),
    ].iter().cloned().collect();

    for line in &mut lines {
        if line.starts_with("TTC_EMAIL=") {
            *line = format!("TTC_EMAIL={}", email);
            *found.get_mut("TTC_EMAIL").unwrap() = true;
        } else if line.starts_with("TTC_PASSKEY=") {
            *line = format!("TTC_PASSKEY={}", passkey);
            *found.get_mut("TTC_PASSKEY").unwrap() = true;
        } else if line.starts_with("TTC_AUTH_TOKEN=") {
            *line = format!("TTC_AUTH_TOKEN={}", auth_token);
            *found.get_mut("TTC_AUTH_TOKEN").unwrap() = true;
        } else if line.starts_with("TTC_PUBLIC_KEY=") {
            *line = format!("TTC_PUBLIC_KEY={}", public_key);
            *found.get_mut("TTC_PUBLIC_KEY").unwrap() = true;
        } else if line.starts_with("TTC_TOKEN_ISSUED_AT=") {
            *line = format!("TTC_TOKEN_ISSUED_AT={}", issued_at);
            *found.get_mut("TTC_TOKEN_ISSUED_AT").unwrap() = true;
        }
        // All other lines (EXCHANGE_*, TTC_EXCHANGE, etc.) are untouched
    }

    if !found["TTC_EMAIL"] { lines.push(format!("TTC_EMAIL={}", email)); }
    if !found["TTC_PASSKEY"] { lines.push(format!("TTC_PASSKEY={}", passkey)); }
    if !found["TTC_AUTH_TOKEN"] { lines.push(format!("TTC_AUTH_TOKEN={}", auth_token)); }
    if !found["TTC_PUBLIC_KEY"] && !public_key.is_empty() {
        lines.push(format!("TTC_PUBLIC_KEY={}", public_key));
    }
    if !found["TTC_TOKEN_ISSUED_AT"] {
        lines.push(format!("TTC_TOKEN_ISSUED_AT={}", issued_at));
    }

    let mut content = lines.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }

    std::fs::write(&env_path, content)
        .map_err(|e| TtcError::Config(format!("Failed to write .env: {}", e)))?;

    Ok(())
}
