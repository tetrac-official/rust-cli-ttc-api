//! Status command — ping TTC Box and verify session validity before starting any loop.
//!
//! Checks:
//!   1. API reachability — raw HTTP GET to a public market endpoint (no auth required)
//!   2. Session validity — TTC_AUTH_TOKEN present + TTC_TOKEN_ISSUED_AT within 24h
//!   3. Exchange credentials — at least one exchange has API key + secret configured
//!
//! Exits with code 0 if READY, code 1 if NOT READY (enables scripting / loop gating).

use crate::config::AppConfig;
use crate::error::Result;
use colored::Colorize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TOKEN_EXPIRY_SECS: u64 = 24 * 3600; // TTC Box sessions expire in 24h

pub async fn execute(settings: &AppConfig) -> Result<()> {
    println!();
    println!("{}", "━".repeat(55));
    println!("  SKILL-TRADING STATUS");
    println!("{}", "━".repeat(55));
    println!();

    let mut all_ok = true;

    // ── 1. API Reachability ────────────────────────────────────────────────
    let base = settings.api.base_url.trim_end_matches('/');
    let ping_url = format!("{}/markets/funding-rates", base);
    let api_ok = check_api_reachability(&ping_url, settings.api.timeout).await;
    print_check("TTC Box API", api_ok, None);
    if !api_ok {
        all_ok = false;
    }

    // ── 2. Session Token ──────────────────────────────────────────────────
    let (session_ok, session_detail) = check_session(settings);
    print_check("Session token", session_ok, Some(&session_detail));
    if !session_ok {
        all_ok = false;
    }

    // ── 3. Exchange credentials ───────────────────────────────────────────
    let (creds_ok, creds_detail) = check_credentials(settings);
    print_check("Exchange credentials", creds_ok, Some(&creds_detail));
    if !creds_ok {
        all_ok = false;
    }

    // ── Summary ───────────────────────────────────────────────────────────
    println!();
    println!("{}", "━".repeat(55));
    if all_ok {
        println!("  STATUS: {}", "READY".green().bold());
    } else {
        println!("  STATUS: {}", "NOT READY".red().bold());
    }
    println!("{}", "━".repeat(55));
    println!();

    if !all_ok {
        // Emit actionable hints
        if !session_ok {
            eprintln!("  Run: skill-trading login");
        }
        if !creds_ok {
            let exchange = settings.exchange.as_deref().unwrap_or("orderly");
            let prefix = exchange.to_uppercase();
            eprintln!("  Set: {}_API_KEY, {}_API_SECRET in .env", prefix, prefix);
        }
        std::process::exit(1);
    }

    Ok(())
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn print_check(label: &str, ok: bool, detail: Option<&str>) {
    let icon = if ok {
        "✓".green().bold()
    } else {
        "✗".red().bold()
    };
    match detail {
        Some(d) => println!("  {}  {:<26}  {}", icon, label, d),
        None => println!("  {}  {}", icon, label),
    }
}

/// Attempt a GET to `url`, returning true if any HTTP response is received.
async fn check_api_reachability(url: &str, timeout_secs: u64) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs.min(10)))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Any HTTP status means the server is reachable (auth errors are fine here)
    client.get(url).send().await.is_ok()
}

/// Check TTC_AUTH_TOKEN presence and TTC_TOKEN_ISSUED_AT recency.
fn check_session(settings: &AppConfig) -> (bool, String) {
    let token_set = settings
        .api_key
        .as_ref()
        .map(|k| !k.is_empty())
        .unwrap_or(false);

    if !token_set {
        return (false, "TTC_AUTH_TOKEN not set".to_string());
    }

    // Check issued-at timestamp if available
    match std::env::var("TTC_TOKEN_ISSUED_AT") {
        Ok(ts_str) => match ts_str.trim().parse::<u64>() {
            Ok(issued_at) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let elapsed = now.saturating_sub(issued_at);

                if elapsed >= TOKEN_EXPIRY_SECS {
                    let hours_ago = elapsed / 3600;
                    (false, format!("EXPIRED ({}h ago)", hours_ago))
                } else {
                    let remaining_secs = TOKEN_EXPIRY_SECS - elapsed;
                    let hours = remaining_secs / 3600;
                    let mins = (remaining_secs % 3600) / 60;
                    (true, format!("VALID  {}h {}m remaining", hours, mins))
                }
            }
            Err(_) => (true, "VALID (issued-at unknown)".to_string()),
        },
        Err(_) => {
            // Token is set but we don't know when it was issued — assume valid
            (true, "VALID (issued-at not recorded)".to_string())
        }
    }
}

/// Check that at least one exchange has API credentials configured.
fn check_credentials(settings: &AppConfig) -> (bool, String) {
    // 1. Global CLI/env override
    if settings.exchange_api_key.is_some() && settings.exchange_api_secret.is_some() {
        return (true, "global EXCHANGE_API_KEY set".to_string());
    }

    // 2. Check the default exchange first
    if let Some(ref exchange) = settings.exchange {
        if settings.get_credentials(exchange).is_some() {
            return (true, format!("{} configured", exchange));
        }
    }

    // 3. Scan config.toml [exchanges] sections
    if !settings.exchanges.is_empty() {
        let names: Vec<&str> = settings.exchanges.keys().map(|k| k.as_str()).collect();
        return (true, format!("{} configured", names.join(", ")));
    }

    // 4. Scan well-known per-exchange env vars
    let known = [
        "ORDERLY", "BYBIT", "OKX", "BINANCE", "BITGET", "BLOFIN", "KUCOIN",
    ];
    for prefix in &known {
        if std::env::var(format!("{}_API_KEY", prefix)).is_ok()
            && std::env::var(format!("{}_API_SECRET", prefix)).is_ok()
        {
            return (true, format!("{} env vars set", prefix));
        }
    }

    (false, "no exchange credentials found".to_string())
}
