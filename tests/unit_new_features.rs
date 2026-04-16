//! Unit tests for new features: market-maker, brief (alerts), and agent loop (twap-slice)

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn cmd() -> Command {
    Command::cargo_bin("skill-trading").unwrap()
}

// ============================================================================
// Market Maker
// ============================================================================

#[test]
fn test_market_maker_help() {
    cmd()
        .args(["market-maker", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("spread"))
        .stdout(predicate::str::contains("quantity"));
}

#[test]
fn test_market_maker_alias_mm() {
    cmd()
        .args(["mm", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("spread"));
}

#[test]
fn test_market_maker_requires_buy_or_sell() {
    cmd()
        .args([
            "market-maker",
            "-e", "orderly",
            "-s", "BTCUSDT",
            "-q", "1",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Must specify --buy or --sell"));
}

#[test]
fn test_market_maker_requires_symbol() {
    cmd()
        .args(["market-maker", "-e", "orderly", "--buy", "-q", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--symbol"));
}

#[test]
fn test_market_maker_requires_quantity() {
    cmd()
        .args([
            "market-maker",
            "-e", "orderly",
            "-s", "BTCUSDT",
            "--buy",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--quantity"));
}

#[test]
fn test_market_maker_buy_sell_conflict() {
    cmd()
        .args([
            "market-maker",
            "-e", "orderly",
            "-s", "BTCUSDT",
            "--buy",
            "--sell",
            "-q", "1",
        ])
        .assert()
        .failure();
}

#[test]
fn test_market_maker_dry_run() {
    cmd()
        .args([
            "--dry-run",
            "market-maker",
            "-e", "orderly",
            "-s", "BTCUSDT",
            "--buy",
            "-q", "1",
            "--rounds", "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY-RUN"));
}

#[test]
fn test_market_maker_spread_pct_default() {
    // With dry-run, verify the default spread_pct (0.1%) appears in header
    cmd()
        .args([
            "--dry-run",
            "market-maker",
            "-e", "orderly",
            "-s", "NEARUSDT",
            "--buy",
            "-q", "100",
            "--rounds", "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("spread: 0.100%"));
}

#[test]
fn test_market_maker_custom_spread_pct() {
    cmd()
        .args([
            "--dry-run",
            "market-maker",
            "-e", "orderly",
            "-s", "NEARUSDT",
            "--sell",
            "-q", "100",
            "--spread-pct", "0.5",
            "--rounds", "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("spread: 0.500%"));
}

#[test]
fn test_market_maker_rounds_header() {
    cmd()
        .args([
            "--dry-run",
            "market-maker",
            "-e", "orderly",
            "-s", "NEARUSDT",
            "--buy",
            "-q", "50",
            "--rounds", "5",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("MARKET MAKER"));
}

// ============================================================================
// Brief (Morning Brief / Alerts)
// ============================================================================

#[test]
fn test_brief_help() {
    cmd()
        .args(["brief", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("exchange"))
        .stdout(predicate::str::contains("watchlist"));
}

#[test]
fn test_brief_alias_morning() {
    cmd()
        .args(["morning", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("exchange"));
}

#[test]
fn test_brief_alias_mb() {
    cmd()
        .args(["mb", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("exchange"));
}

#[test]
fn test_brief_uses_default_exchange_from_config() {
    // Even without TTC_EXCHANGE env var, brief succeeds using config.toml default
    cmd()
        .args(["brief"])
        .env_remove("TTC_EXCHANGE")
        .assert()
        .success()
        .stdout(predicate::str::contains("MORNING BRIEF"));
}

#[test]
fn test_brief_custom_watchlist() {
    // Should accept --watchlist flag without error (may fail on API but the arg parsing succeeds)
    cmd()
        .args([
            "brief",
            "-e", "orderly",
            "--watchlist", "BTCUSDT,ETHUSDT",
        ])
        .assert()
        // The command runs — it prints the header even if API calls fail
        .stdout(predicate::str::contains("MORNING BRIEF"));
}

#[test]
fn test_brief_custom_timeframe() {
    cmd()
        .args([
            "brief",
            "-e", "orderly",
            "--timeframe", "4h",
        ])
        .assert()
        .stdout(predicate::str::contains("MORNING BRIEF"));
}

#[test]
fn test_brief_default_timeframe_1h() {
    // Default timeframe should be 1h — appears in signals section header
    cmd()
        .args(["brief", "-e", "orderly"])
        .assert()
        .stdout(predicate::str::contains("SIGNALS (1h)"));
}

// ============================================================================
// TWAP Slice (Agent Loop Building Block)
// ============================================================================

#[test]
fn test_twap_slice_help() {
    cmd()
        .args(["twap-slice", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("amount"))
        .stdout(predicate::str::contains("decimals"));
}

#[test]
fn test_twap_slice_requires_buy_or_sell() {
    cmd()
        .args([
            "twap-slice",
            "-e", "orderly",
            "-s", "NEARUSDT",
            "--amount", "15",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Must specify --buy or --sell"));
}

#[test]
fn test_twap_slice_requires_symbol() {
    cmd()
        .args(["twap-slice", "-e", "orderly", "--buy", "--amount", "15"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--symbol"));
}

#[test]
fn test_twap_slice_requires_amount() {
    cmd()
        .args(["twap-slice", "-e", "orderly", "-s", "NEARUSDT", "--buy"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--amount"));
}

#[test]
fn test_twap_slice_buy_sell_conflict() {
    cmd()
        .args([
            "twap-slice",
            "-e", "orderly",
            "-s", "NEARUSDT",
            "--buy",
            "--sell",
            "--amount", "15",
        ])
        .assert()
        .failure();
}

#[test]
fn test_twap_slice_dry_run() {
    cmd()
        .args([
            "--dry-run",
            "twap-slice",
            "-e", "orderly",
            "-s", "NEARUSDT",
            "--buy",
            "--amount", "15",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY-RUN"))
        .stdout(predicate::str::contains("twap-slice"));
}

#[test]
fn test_twap_slice_dry_run_with_label() {
    cmd()
        .args([
            "--dry-run",
            "twap-slice",
            "-e", "orderly",
            "-s", "NEARUSDT",
            "--sell",
            "--amount", "20",
            "--label", "3/10",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY-RUN"))
        .stdout(predicate::str::contains("3/10"));
}

#[test]
fn test_twap_slice_dry_run_custom_decimals() {
    cmd()
        .args([
            "--dry-run",
            "twap-slice",
            "-e", "orderly",
            "-s", "BTCUSDT",
            "--buy",
            "--amount", "100",
            "--decimals", "3",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY-RUN"));
}

// ============================================================================
// Cross-feature: status command still works
// ============================================================================

#[test]
fn test_status_help() {
    cmd()
        .args(["status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("session"));
}

// ============================================================================
// Version reflects update
// ============================================================================

#[test]
fn test_version_shows_current() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.2"));
}
