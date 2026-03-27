//! Integration tests for skill-trading CLI

use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("skill-trading").unwrap()
}

#[test]
fn test_help_output() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Execute trading operations on TTC Box"));
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
    cmd()
        .arg("config")
        .arg("path")
        .assert()
        .success()
        .stdout(predicate::str::contains("Config file"));
}

#[test]
fn test_order_requires_buy_or_sell() {
    // Should fail because neither --buy nor --sell specified
    cmd()
        .args(["order", "market", "-e", "phemex", "-s", "BTCUSDT", "-q", "0.001"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Must specify --buy or --sell"));
}

#[test]
fn test_order_limit_requires_buy_or_sell() {
    cmd()
        .args(["order", "limit", "-e", "phemex", "-s", "BTCUSDT", "-q", "0.001", "-p", "50000"])
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
        .args(["account", "leverage", "-e", "phemex", "-s", "BTCUSDT", "-l", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Leverage must be greater than 0"));
}

#[test]
fn test_dry_run_limit_order() {
    cmd()
        .args([
            "--dry-run",
            "order", "limit",
            "-e", "phemex",
            "-s", "BTCUSDT",
            "--buy",
            "-q", "0.001",
            "-p", "50000",
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
            "order", "market",
            "-e", "phemex",
            "-s", "BTCUSDT",
            "--sell",
            "-q", "0.001",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY-RUN"))
        .stdout(predicate::str::contains("market order"));
}

#[test]
fn test_dry_run_cancel_all() {
    cmd()
        .args([
            "--dry-run",
            "order", "cxl-all",
            "-e", "phemex",
        ])
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
            "position", "close",
            "-e", "phemex",
            "-s", "BTCUSDT",
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
            "account", "leverage",
            "-e", "phemex",
            "-s", "BTCUSDT",
            "-l", "20",
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
    cmd()
        .arg("nonexistent")
        .assert()
        .failure();
}
