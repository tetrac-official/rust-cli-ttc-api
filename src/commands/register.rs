//! Register command — create a new TTC Box account with email + passkey.
//!
//! Generates all required wallets client-side, encrypts private keys with
//! the derived apiKey, and POSTs the encrypted blobs to /api/auth/register.
//! On success, writes TTC_AUTH_TOKEN and TTC_PUBLIC_KEY to .env.

use crate::cli::RegisterArgs;
use crate::config::AppConfig;
use crate::crypto::{
    crypto_es_encrypt, derive_api_key, generate_evm_wallet, generate_solana_keypair,
    hash_passkey_for_server,
};
use crate::error::{Result, TtcError};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
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
    orderly: SolanaWallet, // Orderly uses same Ed25519 format as Solana
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
    // Collect email
    let email = if let Some(e) = args.email {
        e
    } else {
        print!("Email: ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| TtcError::Config(format!("Failed to read email: {}", e)))?;
        input.trim().to_string()
    };

    if email.is_empty() {
        return Err(TtcError::Config("Email cannot be empty".to_string()));
    }

    // Collect passkey (hidden)
    let passkey = rpassword::prompt_password("Passkey (choose a strong secret): ")
        .map_err(|e| TtcError::Config(format!("Failed to read passkey: {}", e)))?;

    if passkey.is_empty() {
        return Err(TtcError::Config("Passkey cannot be empty".to_string()));
    }

    let passkey_confirm = rpassword::prompt_password("Confirm passkey: ")
        .map_err(|e| TtcError::Config(format!("Failed to read passkey confirmation: {}", e)))?;

    if passkey != passkey_confirm {
        return Err(TtcError::Config("Passkays do not match".to_string()));
    }

    // --- Key derivation ---
    println!("Generating wallets and encrypting keys...");

    let api_key = derive_api_key(&passkey, &email);
    let hashed_passkey = hash_passkey_for_server(&passkey);

    // --- Generate wallets ---
    let solana_kp = generate_solana_keypair();
    let orderly_kp = generate_solana_keypair();
    let evm_main = generate_evm_wallet();
    let evm_signing = generate_evm_wallet();

    // --- Encrypt private keys ---
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
            address: evm_signing.address,
            encrypted_private_key: crypto_es_encrypt(&evm_signing.private_key, &api_key),
        },
    };

    let body = RegisterRequest {
        email: email.clone(),
        hashed_passkey,
        client_generated_wallets: client_wallets,
    };

    // --- POST to /api/auth/register ---
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

    // Write credentials to .env
    update_env_file(&auth_token, &solana_kp.public_key)?;

    if is_new {
        println!("Registration successful.");
    } else {
        println!("Account already exists — returning existing session.");
    }
    println!("TTC_AUTH_TOKEN and TTC_PUBLIC_KEY written to .env");
    println!("Public key: {}", solana_kp.public_key);
    println!("Token expires in 24 hours.");
    println!();
    println!("IMPORTANT: Keep your passkey safe — it is the only way to recover your wallet keys.");

    Ok(())
}

fn update_env_file(auth_token: &str, public_key: &str) -> Result<()> {
    let env_path = PathBuf::from(".env");

    let existing = if env_path.exists() {
        std::fs::read_to_string(&env_path)
            .map_err(|e| TtcError::Config(format!("Failed to read .env: {}", e)))?
    } else {
        String::new()
    };

    let mut lines: Vec<String> = existing.lines().map(|l| l.to_string()).collect();
    let mut found_token = false;
    let mut found_pubkey = false;

    for line in &mut lines {
        if line.starts_with("TTC_AUTH_TOKEN=") {
            *line = format!("TTC_AUTH_TOKEN={}", auth_token);
            found_token = true;
        } else if line.starts_with("TTC_PUBLIC_KEY=") {
            *line = format!("TTC_PUBLIC_KEY={}", public_key);
            found_pubkey = true;
        }
    }

    if !found_token {
        lines.push(format!("TTC_AUTH_TOKEN={}", auth_token));
    }
    if !found_pubkey {
        lines.push(format!("TTC_PUBLIC_KEY={}", public_key));
    }

    let mut content = lines.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }

    std::fs::write(&env_path, content)
        .map_err(|e| TtcError::Config(format!("Failed to write .env: {}", e)))?;

    Ok(())
}
