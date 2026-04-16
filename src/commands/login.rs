//! Login command — authenticate with TTC Box via email + passkey.
//!
//! Reads TTC_EMAIL and TTC_PASSKEY from .env automatically — no prompts needed
//! if both are set. Updates TTC_AUTH_TOKEN, TTC_PUBLIC_KEY, and
//! TTC_TOKEN_ISSUED_AT on success. Never touches exchange API keys.

use crate::cli::LoginArgs;
use crate::commands::register::update_env_file;
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

#[derive(Debug, Serialize)]
struct LoginRequest {
    email: String,
    #[serde(rename = "passKey")]
    pass_key: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    success: bool,
    #[serde(rename = "authToken")]
    auth_token: Option<String>,
    user: Option<LoginUser>,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginUser {
    #[serde(rename = "publicKey")]
    public_key: String,
}

/// Silently refresh the TTC session token if it is ≥ 23 hours old.
///
/// Returns `Ok(true)` if the token was refreshed, `Ok(false)` if it is still
/// fresh or the credentials needed to refresh are not available, or `Err` if a
/// refresh was attempted but failed.
pub async fn try_silent_refresh(settings: &AppConfig) -> Result<bool> {
    // Read when the current token was issued
    let issued_at: u64 = match std::env::var("TTC_TOKEN_ISSUED_AT")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        Some(t) => t,
        None => return Ok(false), // No timestamp — can't determine age
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let age_secs = now.saturating_sub(issued_at);
    const REFRESH_AFTER_SECS: u64 = 23 * 3600; // refresh within the last hour of validity

    if age_secs < REFRESH_AFTER_SECS {
        return Ok(false); // Token still fresh
    }

    // Need email + passkey to refresh silently
    let email = match std::env::var("TTC_EMAIL")
        .ok()
        .filter(|s| !s.is_empty())
    {
        Some(e) => e,
        None => return Ok(false),
    };

    let pass_key = match std::env::var("TTC_PASSKEY")
        .ok()
        .filter(|s| !s.is_empty())
    {
        Some(p) => p,
        None => return Ok(false),
    };

    let base = settings.api.base_url.trim_end_matches('/');
    let auth_url = if let Some(pos) = base.find("/api/") {
        format!("{}/api/auth/login", &base[..pos])
    } else {
        "https://ttc.box/api/auth/login".to_string()
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(settings.api.timeout))
        .build()
        .map_err(|e| TtcError::Config(format!("Failed to build HTTP client: {}", e)))?;

    let resp = client
        .post(&auth_url)
        .json(&LoginRequest {
            email: email.clone(),
            pass_key: pass_key.clone(),
        })
        .send()
        .await
        .map_err(|e| TtcError::Config(format!("Token refresh request failed: {}", e)))?;

    let status = resp.status().as_u16();
    let body: LoginResponse = resp
        .json()
        .await
        .map_err(|e| TtcError::Config(format!("Failed to parse refresh response: {}", e)))?;

    if !body.success || body.auth_token.is_none() {
        let msg = body
            .error
            .or(body.message)
            .unwrap_or_else(|| format!("Token refresh failed (HTTP {})", status));
        return Err(TtcError::Api { code: status, message: msg });
    }

    let auth_token = body.auth_token.unwrap();
    let public_key = body.user.map(|u| u.public_key).unwrap_or_default();

    update_env_file(&email, &pass_key, &auth_token, &public_key)?;

    // Update in-process env so the rest of this invocation uses the new token
    std::env::set_var("TTC_AUTH_TOKEN", &auth_token);
    std::env::set_var("TTC_TOKEN_ISSUED_AT", now.to_string());
    if !public_key.is_empty() {
        std::env::set_var("TTC_PUBLIC_KEY", &public_key);
    }

    Ok(true)
}

pub async fn execute(args: LoginArgs, settings: &AppConfig) -> Result<()> {
    // Email: clap resolves from --email arg or TTC_EMAIL env var; prompt if still missing
    let email = match args.email {
        Some(e) if !e.is_empty() => e,
        _ => {
            print!("Email: ");
            io::stdout().flush().ok();
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| TtcError::Config(format!("Failed to read email: {}", e)))?;
            input.trim().to_string()
        }
    };

    if email.is_empty() {
        return Err(TtcError::Config("Email cannot be empty".to_string()));
    }

    // Passkey: read from TTC_PASSKEY env var if set, otherwise prompt
    let pass_key = match std::env::var("TTC_PASSKEY") {
        Ok(p) if !p.is_empty() => {
            println!("Using passkey from TTC_PASSKEY.");
            p
        }
        _ => {
            let p = rpassword::prompt_password("Passkey: ")
                .map_err(|e| TtcError::Config(format!("Failed to read passkey: {}", e)))?;
            if p.is_empty() {
                return Err(TtcError::Config("Passkey cannot be empty".to_string()));
            }
            p
        }
    };

    let base = settings.api.base_url.trim_end_matches('/');
    let auth_url = if let Some(pos) = base.find("/api/") {
        format!("{}/api/auth/login", &base[..pos])
    } else {
        "https://ttc.box/api/auth/login".to_string()
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(settings.api.timeout))
        .build()
        .map_err(|e| TtcError::Config(format!("Failed to build HTTP client: {}", e)))?;

    let resp = client
        .post(&auth_url)
        .json(&LoginRequest {
            email: email.clone(),
            pass_key: pass_key.clone(),
        })
        .send()
        .await
        .map_err(|e| TtcError::Config(format!("Login request failed: {}", e)))?;

    let status = resp.status().as_u16();
    let body: LoginResponse = resp
        .json()
        .await
        .map_err(|e| TtcError::Config(format!("Failed to parse login response: {}", e)))?;

    if !body.success || body.auth_token.is_none() {
        let msg = body
            .error
            .or(body.message)
            .unwrap_or_else(|| format!("Login failed (HTTP {})", status));
        return Err(TtcError::Api {
            code: status,
            message: msg,
        });
    }

    let auth_token = body.auth_token.unwrap();
    let public_key = body.user.map(|u| u.public_key).unwrap_or_default();

    update_env_file(&email, &pass_key, &auth_token, &public_key)?;

    println!("Login successful.");
    println!("Updated .env: TTC_AUTH_TOKEN, TTC_TOKEN_ISSUED_AT");
    println!("Token expires in 24 hours.");
    if !public_key.is_empty() {
        println!("Public key: {}", public_key);
    }

    Ok(())
}
