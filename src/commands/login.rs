//! Login command — authenticate with TTC Box via email + passkey

use crate::cli::LoginArgs;
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::PathBuf;

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

pub async fn execute(args: LoginArgs, settings: &AppConfig) -> Result<()> {
    // Get email — prompt if not provided via flag
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

    // Hidden passkey input
    let pass_key = rpassword::prompt_password("Passkey: ")
        .map_err(|e| TtcError::Config(format!("Failed to read passkey: {}", e)))?;

    if pass_key.is_empty() {
        return Err(TtcError::Config("Passkey cannot be empty".to_string()));
    }

    // Derive auth URL from configured base_url
    // base_url is typically "https://ttc.box/api/v1" → auth is at "https://ttc.box/api/auth/login"
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
        .json(&LoginRequest { email, pass_key })
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
        return Err(TtcError::Api { code: status, message: msg });
    }

    let auth_token = body.auth_token.unwrap();
    let public_key = body.user.map(|u| u.public_key).unwrap_or_default();

    update_env_file(&auth_token, &public_key)?;

    println!("Login successful.");
    println!("TTC_AUTH_TOKEN and TTC_PUBLIC_KEY written to .env");
    println!("Token expires in 24 hours.");
    if !public_key.is_empty() {
        println!("Public key: {}", public_key);
    }

    Ok(())
}

/// Update or create .env in the current directory with new TTC credentials.
/// Preserves all other existing lines.
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
    if !found_pubkey && !public_key.is_empty() {
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
