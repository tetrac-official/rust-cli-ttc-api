//! Integration tests for skill-trading CLI

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn cmd() -> Command {
    Command::cargo_bin("skill-trading").unwrap()
}

#[test]
fn test_help_output() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Execute trading operations on TTC Box",
        ));
}

#[test]
fn test_version_output() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("skill-trading"));
}

#[test]
fn test_info_command() {
    cmd()
        .arg("info")
        .assert()
        .success()
        .stdout(predicate::str::contains("skill-trading v"));
}

#[test]
fn test_config_path_command() {
    // "Config file location:" is a status message → stderr.
    // The actual path is data → stdout (always contains the literal "config.toml").
    cmd()
        .arg("config")
        .arg("path")
        .assert()
        .success()
        .stderr(predicate::str::contains("Config file"))
        .stdout(predicate::str::contains("config.toml"));
}

#[test]
fn test_order_requires_buy_or_sell() {
    // Should fail because neither --buy nor --sell specified
    cmd()
        .args([
            "order", "market", "-e", "phemex", "-s", "BTCUSDT", "-q", "0.001",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Must specify --buy or --sell"));
}

#[test]
fn test_order_limit_requires_buy_or_sell() {
    cmd()
        .args([
            "order", "limit", "-e", "phemex", "-s", "BTCUSDT", "-q", "0.001", "-p", "50000",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Must specify --buy or --sell"));
}

#[test]
fn test_cancel_requires_order_id() {
    // Should fail because no --order-id or --client-order-id
    cmd()
        .args(["order", "cxl", "-e", "phemex", "-s", "BTCUSDT"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("order-id or --client-order-id"));
}

#[test]
fn test_hedge_requires_enable_or_disable() {
    cmd()
        .args(["account", "hedge", "-e", "phemex"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--enable or --disable"));
}

#[test]
fn test_leverage_zero_rejected() {
    cmd()
        .args([
            "account", "leverage", "-e", "phemex", "-s", "BTCUSDT", "-l", "0",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Leverage must be greater than 0"));
}

#[test]
fn test_dry_run_limit_order() {
    cmd()
        .args([
            "--dry-run",
            "order",
            "limit",
            "-e",
            "phemex",
            "-s",
            "BTCUSDT",
            "--buy",
            "-q",
            "0.001",
            "-p",
            "50000",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY-RUN"))
        .stdout(predicate::str::contains("limit order"));
}

#[test]
fn test_dry_run_market_order() {
    cmd()
        .args([
            "--dry-run",
            "order",
            "market",
            "-e",
            "phemex",
            "-s",
            "BTCUSDT",
            "--sell",
            "-q",
            "0.001",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY-RUN"))
        .stdout(predicate::str::contains("market order"));
}

#[test]
fn test_dry_run_cancel_all() {
    cmd()
        .args(["--dry-run", "order", "cxl-all", "-e", "phemex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY-RUN"))
        .stdout(predicate::str::contains("cancel all orders"));
}

#[test]
fn test_dry_run_close_position() {
    cmd()
        .args([
            "--dry-run",
            "position",
            "close",
            "-e",
            "phemex",
            "-s",
            "BTCUSDT",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY-RUN"))
        .stdout(predicate::str::contains("close position"));
}

#[test]
fn test_dry_run_set_leverage() {
    cmd()
        .args([
            "--dry-run",
            "account",
            "leverage",
            "-e",
            "phemex",
            "-s",
            "BTCUSDT",
            "-l",
            "20",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY-RUN"))
        .stdout(predicate::str::contains("leverage"));
}

#[test]
fn test_output_format_flag() {
    // JSON output format should be accepted
    cmd()
        .args(["--output-format", "json", "config", "path"])
        .assert()
        .success();
}

#[test]
fn test_invalid_subcommand() {
    cmd().arg("nonexistent").assert().failure();
}

// ============================================================================
// Config priority: CLI flag and TTC_CONFIG env var control which file loads
// ============================================================================

fn write_temp_config(contents: &str) -> std::path::PathBuf {
    use std::io::Write;
    let path = std::env::temp_dir().join(format!(
        "skill-trading-it-{}-{}.toml",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut f = std::fs::File::create(&path).expect("create temp config");
    f.write_all(contents.as_bytes()).expect("write");
    path
}

/// Build a minimally valid config — `[api]`, `[trading]`, `[output]` are
/// required (no #[serde(default)]). Extras get appended verbatim.
fn config_with(base_url: &str, extras: &str) -> String {
    format!(
        r#"
[api]
base_url = "{base_url}"
timeout = 30
max_retries = 3
retry_delay_ms = 1000

[trading]
default_size = 0.001
default_leverage = 10
confirm_orders = true
dry_run = false

[output]
format = "table"
color = true

{extras}
"#
    )
}

// Note: the binary pre-loads `./config.toml` (or the user config) into
// TTC_EXCHANGE before clap parses, so asserting on the `exchange` field is
// unreliable when those discovery files exist. We assert on api.base_url
// instead — it is loaded from the --config file and not pre-injected.

#[test]
fn test_config_flag_loads_explicit_file() {
    let cfg = write_temp_config(&config_with("https://from-flag.example/api", ""));
    cmd()
        .args(["--config", cfg.to_str().unwrap(), "config", "show"])
        .env_remove("TTC_CONFIG")
        .assert()
        .success()
        .stdout(predicate::str::contains("https://from-flag.example/api"));
    let _ = std::fs::remove_file(&cfg);
}

#[test]
fn test_ttc_config_env_loads_file() {
    let cfg = write_temp_config(&config_with("https://from-ttc-config-env.example/api", ""));
    cmd()
        .env("TTC_CONFIG", cfg.to_str().unwrap())
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "https://from-ttc-config-env.example/api",
        ));
    let _ = std::fs::remove_file(&cfg);
}

#[test]
fn test_cli_flag_overrides_ttc_config_env() {
    let cfg_env = write_temp_config(&config_with("https://from-env-path.example/api", ""));
    let cfg_flag = write_temp_config(&config_with("https://from-flag-path.example/api", ""));
    cmd()
        .env("TTC_CONFIG", cfg_env.to_str().unwrap())
        .args(["--config", cfg_flag.to_str().unwrap(), "config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "https://from-flag-path.example/api",
        ))
        .stdout(predicate::str::contains("https://from-env-path.example/api").not());
    let _ = std::fs::remove_file(&cfg_env);
    let _ = std::fs::remove_file(&cfg_flag);
}

#[test]
fn test_ttc_exchange_env_overrides_config_file() {
    // env should beat config.toml's `exchange` field.
    let cfg = write_temp_config(&config_with(
        "https://ttc.box/api/v1",
        r#"exchange = "from-config-toml""#,
    ));
    cmd()
        .env("TTC_CONFIG", cfg.to_str().unwrap())
        .env("TTC_EXCHANGE", "from-env-var")
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("from-env-var"))
        .stdout(predicate::str::contains("from-config-toml").not());
    let _ = std::fs::remove_file(&cfg);
}

#[test]
fn test_malformed_config_file_fails_clearly() {
    let cfg = write_temp_config("this is not = = valid toml [[");
    cmd()
        .args(["--config", cfg.to_str().unwrap(), "config", "show"])
        .env_remove("TTC_EXCHANGE")
        .env_remove("TTC_CONFIG")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to parse config file"));
    let _ = std::fs::remove_file(&cfg);
}
