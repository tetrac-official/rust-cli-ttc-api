//! Output formatter tests — drive the binary against a mocked TTC Box and
//! assert stdout shape across `--output-format` values.
//!
//! `Tableable::print_*` writes via `println!`, which can't be captured
//! in-process easily, so we exercise the dispatch through the real CLI
//! subprocess via assert_cmd + mockito.

// Holding the SERIAL std::sync::Mutex across `.await` is the deliberate
// behavior — we need each test to keep the lock until its subprocess +
// mockito server finish so they don't race. Single-threaded contention
// only; no deadlock risk.
#![allow(clippy::await_holding_lock)]

use assert_cmd::Command;
use mockito::Server;
use predicates::prelude::*;
use std::io::Write;
use std::sync::Mutex;

/// Serializes the subprocess tests in this file. With 9 parallel tests each
/// starting a mockito server AND spawning the binary, the system occasionally
/// fails to wire up a connection (port allocation race / process slot
/// exhaustion). The lock keeps the tests in this binary sequential without
/// forcing the whole suite to --test-threads=1.
static SERIAL: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|p| p.into_inner())
}

const ONE_BALANCE: &str =
    r#"{"success":true,"data":[{"asset":"USDT","balance":100.0,"available":80.0,"locked":20.0}]}"#;

const EMPTY_BALANCE: &str = r#"{"success":true,"data":[]}"#;

fn write_temp_config(server_url: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "skill-trading-output-{}-{}.toml",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
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

#[allow(deprecated)]
fn cmd(cfg_path: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("skill-trading").unwrap();
    c.env("TTC_CONFIG", cfg_path)
        .env("TTC_AUTH_TOKEN", "test-token")
        .env("TTC_PUBLIC_KEY", "test-public")
        .env("ORDERLY_API_KEY", "k")
        .env("ORDERLY_API_SECRET", "s")
        .env("ORDERLY_API_PASSPHRASE", "p")
        // Block silent token-refresh so it doesn't try to hit the real API.
        .env_remove("TTC_TOKEN_ISSUED_AT")
        .env_remove("TTC_EMAIL")
        .env_remove("TTC_PASSKEY")
        // Drop NO_COLOR — clap rejects "1" for --no-color: bool. The colored
        // crate already auto-disables when stdout is a pipe (assert_cmd
        // captures via pipe), so we don't need to set it.
        .env_remove("NO_COLOR");
    c
}

// ============================================================================
// JSON
// ============================================================================

#[tokio::test]
async fn json_output_emits_parseable_json_array() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(ONE_BALANCE)
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    let out = cmd(&cfg)
        .args([
            "--output-format",
            "json",
            "account",
            "balance",
            "-e",
            "orderly",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(out).unwrap();

    // After the structured-output fix, stdout is ENTIRELY valid JSON —
    // status messages and tracing logs go to stderr. An agent can pipe
    // stdout straight to jq.
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("entire stdout must be valid JSON");
    let arr = parsed.as_array().expect("expected an array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["asset"], "USDT");
    assert_eq!(arr[0]["balance"], 100.0);

    let _ = std::fs::remove_file(&cfg);
}

// ============================================================================
// CSV
// ============================================================================

#[tokio::test]
async fn csv_output_starts_with_header() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(ONE_BALANCE)
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    cmd(&cfg)
        .args([
            "--output-format",
            "csv",
            "account",
            "balance",
            "-e",
            "orderly",
        ])
        .assert()
        .success()
        // Header line for Tableable<Balance>::print_csv_header
        .stdout(predicate::str::contains("asset,balance,available,locked"))
        // Data line
        .stdout(predicate::str::contains("USDT,100,80,20"));

    let _ = std::fs::remove_file(&cfg);
}

#[tokio::test]
async fn csv_output_data_row_field_count_matches_header() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(ONE_BALANCE)
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    let out = cmd(&cfg)
        .args([
            "--output-format",
            "csv",
            "account",
            "balance",
            "-e",
            "orderly",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(out).unwrap();

    let header_line = stdout
        .lines()
        .find(|l| l.contains("asset,balance"))
        .expect("header missing");
    let data_line = stdout
        .lines()
        .find(|l| l.starts_with("USDT,"))
        .expect("data missing");
    assert_eq!(
        header_line.split(',').count(),
        data_line.split(',').count(),
        "header/row column count mismatch"
    );

    let _ = std::fs::remove_file(&cfg);
}

// ============================================================================
// Quiet
// ============================================================================

#[tokio::test]
async fn quiet_output_emits_minimal_asset_balance_pair() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(ONE_BALANCE)
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    let out = cmd(&cfg)
        .args([
            "--output-format",
            "quiet",
            "account",
            "balance",
            "-e",
            "orderly",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(out).unwrap();

    // print_quiet for Balance is `println!("{}:{}", asset, balance)`.
    assert!(
        stdout.lines().any(|l| l.trim() == "USDT:100"),
        "quiet line not found in:\n{stdout}"
    );
    // The format-dispatched output should NOT include table/CSV/JSON markers.
    // (Tracing log lines and the unconditional `printer.info` header still
    // print regardless of format — that's a separate issue, not a quiet
    // dispatch bug.)
    assert!(
        !stdout.contains("BAL USDT"),
        "table data row leaked into quiet"
    );
    assert!(
        !stdout.contains("asset,balance,available,locked"),
        "csv header leaked into quiet"
    );
    // Pretty-printed JSON has `[` alone on a line — quiet must not produce that.
    assert!(
        !stdout.lines().any(|l| l.trim() == "["),
        "JSON array dispatch leaked into quiet"
    );

    let _ = std::fs::remove_file(&cfg);
}

// ============================================================================
// Table (default) — verify it's distinctive vs the others
// ============================================================================

#[tokio::test]
async fn table_output_uses_bal_marker() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(ONE_BALANCE)
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    cmd(&cfg)
        .args([
            "--output-format",
            "table",
            "account",
            "balance",
            "-e",
            "orderly",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("BAL"))
        .stdout(predicate::str::contains("USDT"))
        .stdout(predicate::str::contains("Available:"));

    let _ = std::fs::remove_file(&cfg);
}

// ============================================================================
// Empty list — "No results found" except in Quiet
// ============================================================================

#[tokio::test]
async fn empty_list_prints_no_results_found_in_table() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(EMPTY_BALANCE)
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    cmd(&cfg)
        .args([
            "--output-format",
            "table",
            "account",
            "balance",
            "-e",
            "orderly",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("No results found"));

    let _ = std::fs::remove_file(&cfg);
}

#[tokio::test]
async fn empty_list_in_quiet_mode_does_not_print_no_results_found() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(EMPTY_BALANCE)
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    cmd(&cfg)
        .args([
            "--output-format",
            "quiet",
            "account",
            "balance",
            "-e",
            "orderly",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("No results found").not());

    let _ = std::fs::remove_file(&cfg);
}

// ============================================================================
// TTC_OUTPUT env + CLI flag override priority
// ============================================================================

#[tokio::test]
async fn ttc_output_env_selects_format() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(ONE_BALANCE)
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    cmd(&cfg)
        .env("TTC_OUTPUT", "csv")
        .args(["account", "balance", "-e", "orderly"])
        .assert()
        .success()
        .stdout(predicate::str::contains("asset,balance,available,locked"));

    let _ = std::fs::remove_file(&cfg);
}

#[tokio::test]
async fn no_color_env_value_one_does_not_break_the_binary() {
    // NO_COLOR=1 (the de-facto standard from no-color.org) used to error out
    // because clap parses --no-color: bool strictly. We dropped the env
    // binding so the binary runs cleanly with NO_COLOR set to any value.
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(ONE_BALANCE)
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    cmd(&cfg)
        .env("NO_COLOR", "1")
        .args([
            "--output-format",
            "json",
            "account",
            "balance",
            "-e",
            "orderly",
        ])
        .assert()
        .success();

    cmd(&cfg)
        .env("NO_COLOR", "true")
        .args(["account", "balance", "-e", "orderly"])
        .assert()
        .success();

    let _ = std::fs::remove_file(&cfg);
}

#[tokio::test]
async fn json_stdout_is_pristine_no_status_or_log_lines() {
    // Locks in the agent-friendly contract: in JSON mode the entire stdout
    // parses as JSON, with no preamble lines.
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(ONE_BALANCE)
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    let out = cmd(&cfg)
        .args([
            "--output-format",
            "json",
            "account",
            "balance",
            "-e",
            "orderly",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(out).unwrap();

    let trimmed = stdout.trim();
    assert!(
        trimmed.starts_with('['),
        "stdout must begin with the JSON array, got:\n{stdout}"
    );
    let _: serde_json::Value = serde_json::from_str(trimmed).expect("entire stdout parses as JSON");

    let _ = std::fs::remove_file(&cfg);
}

#[tokio::test]
async fn csv_stdout_is_pristine_first_line_is_header() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(ONE_BALANCE)
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    let out = cmd(&cfg)
        .args([
            "--output-format",
            "csv",
            "account",
            "balance",
            "-e",
            "orderly",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(out).unwrap();
    let first_line = stdout.lines().next().expect("at least one line");
    assert_eq!(
        first_line, "asset,balance,available,locked",
        "CSV header must be the first stdout line; preamble would break parsers"
    );

    let _ = std::fs::remove_file(&cfg);
}

#[tokio::test]
async fn quiet_stdout_is_pristine_only_data_lines() {
    let _g = lock();
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(ONE_BALANCE)
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    let out = cmd(&cfg)
        .args([
            "--output-format",
            "quiet",
            "account",
            "balance",
            "-e",
            "orderly",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(out).unwrap();
    let data: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        data,
        vec!["USDT:100"],
        "quiet stdout must contain ONLY the data lines, got: {stdout:?}"
    );

    let _ = std::fs::remove_file(&cfg);
}

#[tokio::test]
async fn cli_flag_overrides_ttc_output_env() {
    let _g = lock();
    // env says csv, flag says json — flag wins.
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/exchanges")
        .with_status(200)
        .with_body(ONE_BALANCE)
        .create_async()
        .await;
    let cfg = write_temp_config(&server.url());

    cmd(&cfg)
        .env("TTC_OUTPUT", "csv")
        .args([
            "--output-format",
            "json",
            "account",
            "balance",
            "-e",
            "orderly",
        ])
        .assert()
        .success()
        // CSV header must NOT appear; JSON object key must appear
        .stdout(predicate::str::contains("asset,balance,available,locked").not())
        .stdout(predicate::str::contains("\"asset\""));

    let _ = std::fs::remove_file(&cfg);
}
