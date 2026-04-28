//! Status command exit-code tests.
//!
//! CLAUDE.md says `status` exits 0 when READY, 1 when NOT READY. Locks in
//! that contract end-to-end so an agent can rely on shell-script gating
//! (e.g. `skill-trading status && skill-trading order ...`).

// SERIAL is std::sync::Mutex held across `.await` deliberately — keeps
// each test holding the lock until subprocess + mockito finish so env
// vars set/removed by one test can't leak into another. No deadlock risk
// here because tests are independent and each test takes/releases once.
#![allow(clippy::await_holding_lock)]

use assert_cmd::Command;
use mockito::Server;
use predicates::prelude::*;
use std::io::Write;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Serial lock — status spawns subprocesses and inspects shared env state;
/// run sequentially within this test binary to avoid env contamination.
static SERIAL: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|p| p.into_inner())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn write_temp_config(server_url: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "skill-trading-status-{}-{}.toml",
        std::process::id(),
        now_secs(),
    ));
    let contents = format!(
        r#"
[api]
base_url = "{server_url}"
timeout = 5
max_retries = 0
retry_delay_ms = 1

[trading]
default_size = 0.001
default_leverage = 10
confirm_orders = true
dry_run = false

[output]
format = "table"
color = true
"#
    );
    std::fs::File::create(&path)
        .unwrap()
        .write_all(contents.as_bytes())
        .unwrap();
    path
}

/// Build a temp directory that has NO `.env` file. main.rs calls
/// `dotenvy::dotenv()` from cwd at startup, so running from the project
/// directory leaks production `.env` vars (real TTC_AUTH_TOKEN, ORDERLY_*)
/// into the test process. Running from an empty temp dir keeps the test
/// deterministic.
fn empty_cwd() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "skill-trading-status-cwd-{}-{}",
        std::process::id(),
        now_secs(),
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[allow(deprecated)]
fn cmd(cfg_path: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("skill-trading").unwrap();
    c.current_dir(empty_cwd())
        .env("TTC_CONFIG", cfg_path)
        // Default: clean slate for env vars status reads.
        .env_remove("TTC_AUTH_TOKEN")
        .env_remove("TTC_PUBLIC_KEY")
        .env_remove("TTC_TOKEN_ISSUED_AT")
        .env_remove("TTC_EMAIL")
        .env_remove("TTC_PASSKEY")
        .env_remove("ORDERLY_API_KEY")
        .env_remove("ORDERLY_API_SECRET")
        .env_remove("ORDERLY_API_PASSPHRASE")
        .env_remove("BYBIT_API_KEY")
        .env_remove("BYBIT_API_SECRET")
        .env_remove("OKX_API_KEY")
        .env_remove("OKX_API_SECRET")
        .env_remove("BINANCE_API_KEY")
        .env_remove("BINANCE_API_SECRET")
        .env_remove("BITGET_API_KEY")
        .env_remove("BITGET_API_SECRET")
        .env_remove("BLOFIN_API_KEY")
        .env_remove("BLOFIN_API_SECRET")
        .env_remove("KUCOIN_API_KEY")
        .env_remove("KUCOIN_API_SECRET")
        .env_remove("NO_COLOR");
    c
}

// ============================================================================
// Exit code 1 paths
// ============================================================================

#[tokio::test]
async fn status_exits_1_with_no_session_or_creds() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/markets/funding-rates")
        .with_status(200)
        .with_body("[]")
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    cmd(&cfg)
        .arg("status")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("NOT READY"));

    let _ = std::fs::remove_file(&cfg);
}

#[tokio::test]
async fn status_exits_1_with_expired_token() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/markets/funding-rates")
        .with_status(200)
        .with_body("[]")
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    let expired = (now_secs() - 25 * 3600).to_string();
    cmd(&cfg)
        .env("TTC_AUTH_TOKEN", "test-token")
        .env("TTC_PUBLIC_KEY", "test-public")
        .env("TTC_TOKEN_ISSUED_AT", &expired)
        .env("ORDERLY_API_KEY", "k")
        .env("ORDERLY_API_SECRET", "s")
        .arg("status")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("EXPIRED"))
        .stdout(predicate::str::contains("NOT READY"));

    let _ = std::fs::remove_file(&cfg);
}

#[tokio::test]
async fn status_exits_1_when_creds_missing_even_with_valid_session() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/markets/funding-rates")
        .with_status(200)
        .with_body("[]")
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    cmd(&cfg)
        .env("TTC_AUTH_TOKEN", "test-token")
        .env("TTC_PUBLIC_KEY", "test-public")
        // No exchange creds → NOT READY
        .arg("status")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("NOT READY"));

    let _ = std::fs::remove_file(&cfg);
}

#[tokio::test]
async fn status_emits_actionable_hints_on_failure() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/markets/funding-rates")
        .with_status(200)
        .with_body("[]")
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    let out = cmd(&cfg)
        .arg("status")
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(out).unwrap();
    // Status hints help an agent / user fix the issue without parsing stdout.
    assert!(
        stderr.contains("skill-trading login") || stderr.contains("API_KEY"),
        "expected actionable hint in stderr, got:\n{stderr}"
    );

    let _ = std::fs::remove_file(&cfg);
}

// ============================================================================
// Exit code 0 path
// ============================================================================

#[tokio::test]
async fn status_exits_0_when_session_creds_and_api_all_ok() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/markets/funding-rates")
        .with_status(200)
        .with_body("[]")
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    let recent = (now_secs() - 3600).to_string(); // 1h ago
    cmd(&cfg)
        .env("TTC_AUTH_TOKEN", "test-token")
        .env("TTC_PUBLIC_KEY", "test-public")
        .env("TTC_TOKEN_ISSUED_AT", &recent)
        .env("ORDERLY_API_KEY", "k")
        .env("ORDERLY_API_SECRET", "s")
        .arg("status")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("READY"))
        // The shows-remaining line confirms the issued-at branch fired.
        .stdout(predicate::str::contains("remaining"));

    let _ = std::fs::remove_file(&cfg);
}

#[tokio::test]
async fn status_exits_0_when_token_present_without_issued_at() {
    // Tokens issued before TTC_TOKEN_ISSUED_AT was tracked should still
    // count as valid — otherwise an upgrade locks users out.
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/markets/funding-rates")
        .with_status(200)
        .with_body("[]")
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    cmd(&cfg)
        .env("TTC_AUTH_TOKEN", "test-token")
        .env("TTC_PUBLIC_KEY", "test-public")
        .env("ORDERLY_API_KEY", "k")
        .env("ORDERLY_API_SECRET", "s")
        // No TTC_TOKEN_ISSUED_AT
        .arg("status")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("READY"));

    let _ = std::fs::remove_file(&cfg);
}

#[tokio::test]
async fn status_exits_1_when_api_unreachable_even_with_valid_creds() {
    // Point at an unreachable port — API check fails, status reports NOT READY
    // even when session + creds are good.
    let _g = lock();
    let cfg_path = std::env::temp_dir().join(format!(
        "skill-trading-status-unreachable-{}-{}.toml",
        std::process::id(),
        now_secs(),
    ));
    let contents = r#"
[api]
base_url = "http://127.0.0.1:1"
timeout = 1
max_retries = 0
retry_delay_ms = 1

[trading]
default_size = 0.001
default_leverage = 10
confirm_orders = true
dry_run = false

[output]
format = "table"
color = true
"#;
    std::fs::write(&cfg_path, contents).unwrap();

    let recent = (now_secs() - 3600).to_string();
    cmd(&cfg_path)
        .env("TTC_AUTH_TOKEN", "t")
        .env("TTC_PUBLIC_KEY", "p")
        .env("TTC_TOKEN_ISSUED_AT", &recent)
        .env("ORDERLY_API_KEY", "k")
        .env("ORDERLY_API_SECRET", "s")
        .arg("status")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("NOT READY"));

    let _ = std::fs::remove_file(&cfg_path);
}
