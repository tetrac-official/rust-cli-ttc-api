//! Status command — ping Tetrac and verify session validity before starting any loop.
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

const TOKEN_EXPIRY_SECS: u64 = 24 * 3600; // Tetrac sessions expire in 24h

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
    print_check("Tetrac API", api_ok, None);
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

#[cfg(test)]
mod tests {
    //! Pure-function tests for status checks. Subprocess tests for the full
    //! command (exit codes 0/1) live in tests/status_test.rs.

    use super::*;
    use crate::commands::common::TEST_ENV_LOCK;
    use crate::config::AppConfig;

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Wraps an env-mutating test in the shared serial lock and a save/restore.
    struct EnvSandbox {
        saved: Vec<(&'static str, Option<String>)>,
        _guard: std::sync::MutexGuard<'static, ()>,
    }

    impl EnvSandbox {
        fn new(keys: &[&'static str]) -> Self {
            let guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
            let saved = keys.iter().map(|k| (*k, std::env::var(k).ok())).collect();
            for k in keys {
                std::env::remove_var(k);
            }
            Self {
                saved,
                _guard: guard,
            }
        }
    }

    impl Drop for EnvSandbox {
        fn drop(&mut self) {
            for (k, v) in &self.saved {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
    }

    fn cfg_with_token(token: Option<&str>) -> AppConfig {
        AppConfig {
            api_key: token.map(String::from),
            ..Default::default()
        }
    }

    // ─── check_session ───────────────────────────────────────────────────────

    #[test]
    fn session_invalid_when_token_missing() {
        let _e = EnvSandbox::new(&["TTC_TOKEN_ISSUED_AT"]);
        let (ok, msg) = check_session(&cfg_with_token(None));
        assert!(!ok);
        assert!(msg.contains("TTC_AUTH_TOKEN not set"), "got: {msg}");
    }

    #[test]
    fn session_invalid_when_token_is_empty_string() {
        let _e = EnvSandbox::new(&["TTC_TOKEN_ISSUED_AT"]);
        let (ok, msg) = check_session(&cfg_with_token(Some("")));
        assert!(!ok);
        assert!(msg.contains("not set"));
    }

    #[test]
    fn session_valid_when_token_set_and_no_issued_at() {
        let _e = EnvSandbox::new(&["TTC_TOKEN_ISSUED_AT"]);
        let (ok, msg) = check_session(&cfg_with_token(Some("any-token")));
        assert!(ok);
        assert!(msg.contains("VALID"));
        assert!(msg.contains("issued-at not recorded"));
    }

    #[test]
    fn session_valid_with_recent_issued_at() {
        let _e = EnvSandbox::new(&["TTC_TOKEN_ISSUED_AT"]);
        let recent = now_secs().saturating_sub(3600); // 1 hour ago
        std::env::set_var("TTC_TOKEN_ISSUED_AT", recent.to_string());
        let (ok, msg) = check_session(&cfg_with_token(Some("t")));
        assert!(ok);
        assert!(msg.contains("VALID"), "got: {msg}");
        assert!(msg.contains("remaining"));
    }

    #[test]
    fn session_expired_when_issued_at_is_25_hours_ago() {
        let _e = EnvSandbox::new(&["TTC_TOKEN_ISSUED_AT"]);
        let expired = now_secs().saturating_sub(25 * 3600);
        std::env::set_var("TTC_TOKEN_ISSUED_AT", expired.to_string());
        let (ok, msg) = check_session(&cfg_with_token(Some("t")));
        assert!(!ok);
        assert!(msg.contains("EXPIRED"), "got: {msg}");
    }

    #[test]
    fn session_boundary_just_under_24h_is_valid() {
        let _e = EnvSandbox::new(&["TTC_TOKEN_ISSUED_AT"]);
        // 23h 59m ago — must still be valid.
        let issued = now_secs().saturating_sub(TOKEN_EXPIRY_SECS - 60);
        std::env::set_var("TTC_TOKEN_ISSUED_AT", issued.to_string());
        let (ok, _) = check_session(&cfg_with_token(Some("t")));
        assert!(ok, "≤ 24h must be valid");
    }

    #[test]
    fn session_boundary_at_24h_exact_is_expired() {
        let _e = EnvSandbox::new(&["TTC_TOKEN_ISSUED_AT"]);
        // Exactly 24h ago — code uses `>=` so this is expired.
        let issued = now_secs().saturating_sub(TOKEN_EXPIRY_SECS);
        std::env::set_var("TTC_TOKEN_ISSUED_AT", issued.to_string());
        let (ok, _) = check_session(&cfg_with_token(Some("t")));
        assert!(!ok, "≥ 24h must be expired");
    }

    #[test]
    fn session_valid_when_issued_at_unparseable() {
        let _e = EnvSandbox::new(&["TTC_TOKEN_ISSUED_AT"]);
        std::env::set_var("TTC_TOKEN_ISSUED_AT", "not-a-number");
        let (ok, msg) = check_session(&cfg_with_token(Some("t")));
        assert!(ok, "malformed timestamp must not lock the user out");
        assert!(msg.contains("issued-at unknown"));
    }

    // ─── check_credentials ──────────────────────────────────────────────────

    /// Use a fictitious exchange to dodge real-exchange env var collisions.
    /// "STATUSEXA" is highly unlikely to be a real exchange name.
    const FICTIONAL_EXCHANGE: &str = "statusexa";

    #[test]
    fn credentials_invalid_when_nothing_set() {
        let _e = EnvSandbox::new(&[
            "ORDERLY_API_KEY",
            "ORDERLY_API_SECRET",
            "BYBIT_API_KEY",
            "BYBIT_API_SECRET",
            "OKX_API_KEY",
            "OKX_API_SECRET",
            "BINANCE_API_KEY",
            "BINANCE_API_SECRET",
            "BITGET_API_KEY",
            "BITGET_API_SECRET",
            "BLOFIN_API_KEY",
            "BLOFIN_API_SECRET",
            "KUCOIN_API_KEY",
            "KUCOIN_API_SECRET",
        ]);
        let (ok, msg) = check_credentials(&AppConfig::default());
        assert!(!ok, "no creds anywhere → invalid");
        assert!(msg.contains("no exchange credentials found"));
    }

    #[test]
    fn credentials_valid_with_global_cli_creds() {
        let _e = EnvSandbox::new(&[]);
        let c = AppConfig {
            exchange_api_key: Some("g-key".into()),
            exchange_api_secret: Some("g-secret".into()),
            ..Default::default()
        };
        let (ok, msg) = check_credentials(&c);
        assert!(ok);
        assert!(msg.contains("global"));
    }

    #[test]
    fn credentials_valid_with_default_exchange_in_config() {
        let _e = EnvSandbox::new(&["BYBIT_API_KEY", "BYBIT_API_SECRET"]);
        let mut c = AppConfig {
            exchange: Some("bybit".into()),
            ..Default::default()
        };
        c.exchanges.insert(
            "bybit".into(),
            crate::config::ExchangeCredentialConfig {
                api_key: "k".into(),
                api_secret: "s".into(),
                passphrase: None,
            },
        );
        let (ok, msg) = check_credentials(&c);
        assert!(ok);
        assert!(msg.contains("bybit"));
    }

    #[test]
    fn credentials_valid_with_per_exchange_env_vars() {
        // Fictional exchange so we don't collide with real env on the host.
        // But check_credentials only scans a hardcoded list of well-known
        // exchanges — so we use ORDERLY here, save/restore.
        let _e = EnvSandbox::new(&["ORDERLY_API_KEY", "ORDERLY_API_SECRET"]);
        std::env::set_var("ORDERLY_API_KEY", "k");
        std::env::set_var("ORDERLY_API_SECRET", "s");
        let (ok, msg) = check_credentials(&AppConfig::default());
        assert!(ok);
        assert!(msg.contains("ORDERLY"));
    }

    #[test]
    fn credentials_partial_env_does_not_count() {
        // KEY without SECRET → not enough on its own.
        let _e = EnvSandbox::new(&[
            "ORDERLY_API_KEY",
            "ORDERLY_API_SECRET",
            "BYBIT_API_KEY",
            "BYBIT_API_SECRET",
            "OKX_API_KEY",
            "OKX_API_SECRET",
            "BINANCE_API_KEY",
            "BINANCE_API_SECRET",
            "BITGET_API_KEY",
            "BITGET_API_SECRET",
            "BLOFIN_API_KEY",
            "BLOFIN_API_SECRET",
            "KUCOIN_API_KEY",
            "KUCOIN_API_SECRET",
        ]);
        std::env::set_var("ORDERLY_API_KEY", "k");
        // Note: no ORDERLY_API_SECRET
        let (ok, _) = check_credentials(&AppConfig::default());
        assert!(!ok, "partial env should not pass");
    }

    #[test]
    fn credentials_valid_when_exchanges_table_has_any_entry() {
        let _e = EnvSandbox::new(&["ORDERLY_API_KEY", "ORDERLY_API_SECRET"]);
        let mut c = AppConfig::default();
        // Default exchange not set, but config has [exchanges.foo]
        c.exchanges.insert(
            FICTIONAL_EXCHANGE.into(),
            crate::config::ExchangeCredentialConfig {
                api_key: "k".into(),
                api_secret: "s".into(),
                passphrase: None,
            },
        );
        let (ok, msg) = check_credentials(&c);
        assert!(ok);
        assert!(msg.contains(FICTIONAL_EXCHANGE));
    }
}
