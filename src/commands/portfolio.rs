//! Portfolio health report — aggregates balance + all positions into one view.

use crate::api::Client;
use crate::cli::{PortfolioCommands, PortfolioSubcommands};
use crate::commands::common::get_credentials;
use crate::config::{AppConfig, PortfolioConfig};
use crate::error::Result;
use crate::models::Position;
use crate::output::OutputFormat;
use colored::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum HealthLevel {
    Healthy,
    Watch,
    Danger,
}

/// Distance from mark to liquidation as a positive percentage. Returns 100%
/// (effectively "no risk signal") when liq or mark is missing/zero — that
/// matches the inline behavior, and the agent should not be alarmed by a
/// missing field.
pub(crate) fn liq_distance_pct(mark_price: f64, liq_price: f64) -> f64 {
    if liq_price > 0.0 && mark_price > 0.0 {
        ((mark_price - liq_price) / mark_price * 100.0).abs()
    } else {
        100.0
    }
}

/// Margin utilization as a percentage of total balance.
pub(crate) fn utilization_pct(total_balance: f64, locked: f64) -> f64 {
    if total_balance > 0.0 {
        locked / total_balance * 100.0
    } else {
        0.0
    }
}

/// Classify portfolio health and return the warnings that triggered it.
///
/// Rules (each comparison is strict — exactly-on-the-threshold values stay
/// HEALTHY):
/// - `utilization > max_margin_utilization`         → WATCH
/// - `notional > max_position_notional` (any pos)   → WATCH
/// - `liq_distance < min_liq_distance_pct` (any pos) → DANGER
///
/// DANGER overrides WATCH; WATCH overrides HEALTHY.
pub(crate) fn classify_health(
    total_balance: f64,
    locked: f64,
    positions: &[Position],
    cfg: &PortfolioConfig,
) -> (HealthLevel, Vec<String>) {
    let mut warnings: Vec<String> = vec![];
    let mut health = HealthLevel::Healthy;

    let util = utilization_pct(total_balance, locked);
    if util > cfg.max_margin_utilization {
        warnings.push(format!(
            "Margin utilization {:.1}% exceeds threshold ({:.1}%)",
            util, cfg.max_margin_utilization
        ));
        if health < HealthLevel::Watch {
            health = HealthLevel::Watch;
        }
    }

    for pos in positions {
        let liq = pos.liquidation_price.unwrap_or(0.0);
        let liq_dist = liq_distance_pct(pos.mark_price, liq);
        if liq_dist < cfg.min_liq_distance_pct {
            warnings.push(format!(
                "{} liq distance {:.2}% is below threshold ({:.1}%)",
                pos.symbol, liq_dist, cfg.min_liq_distance_pct
            ));
            if health < HealthLevel::Danger {
                health = HealthLevel::Danger;
            }
        }

        let notional = pos.notional.unwrap_or(0.0);
        if notional > cfg.max_position_notional {
            warnings.push(format!(
                "{} notional ${:.2} exceeds max_position_notional (${:.2})",
                pos.symbol, notional, cfg.max_position_notional
            ));
            if health < HealthLevel::Watch {
                health = HealthLevel::Watch;
            }
        }
    }

    (health, warnings)
}

pub async fn execute(
    cmd: PortfolioCommands,
    settings: &AppConfig,
    _format: OutputFormat,
) -> Result<()> {
    match cmd.command {
        PortfolioSubcommands::Summary(args) => {
            let client = Client::new(settings)?;
            let credentials = get_credentials(
                &args.exchange,
                args.api_key,
                args.api_secret,
                args.passphrase,
                settings,
            )?;

            // Fetch balance and positions concurrently
            let (balances, positions) = tokio::try_join!(
                client.get_balance(&args.exchange, credentials.clone()),
                client.get_positions(&args.exchange, None, credentials.clone()),
            )?;

            let cfg = &settings.portfolio;

            // Pull USDT balance
            let bal = balances.iter().find(|b| b.asset.to_uppercase() == "USDT");
            let total_balance = bal.map(|b| b.balance).unwrap_or(0.0);
            let available = bal.map(|b| b.available).unwrap_or(0.0);
            let locked = bal.and_then(|b| b.locked).unwrap_or(0.0);
            let util = utilization_pct(total_balance, locked);
            let warn_utilization = util > cfg.max_margin_utilization;

            // Single source of truth for health classification + warnings.
            let (health, warnings) = classify_health(total_balance, locked, &positions, cfg);

            let mut total_notional = 0.0_f64;
            let mut total_pnl = 0.0_f64;
            let mut total_margin = 0.0_f64;

            println!();
            println!("  {}", "━".repeat(55));
            println!("  PORTFOLIO SUMMARY — {}", args.exchange.to_uppercase());
            println!("  {}", "━".repeat(55));

            // Balance section
            println!();
            println!("  ACCOUNT BALANCE");
            println!("  Total:        ${:.2} USDT", total_balance);
            println!("  Available:    ${:.2}", available);
            println!("  Locked:       ${:.2}", locked);
            if warn_utilization {
                println!(
                    "  Utilization:  {:.1}%  {}",
                    util,
                    format!("[WATCH — threshold {:.0}%]", cfg.max_margin_utilization).yellow()
                );
            } else {
                println!("  Utilization:  {:.1}%", util);
            }

            // Positions section
            println!();
            if positions.is_empty() {
                println!("  POSITIONS — none open");
            } else {
                println!("  POSITIONS ({} open)", positions.len());
                println!("  {}", "─".repeat(55));

                for pos in &positions {
                    let pnl = pos.effective_pnl();
                    let notional = pos.notional.unwrap_or(0.0);
                    let pnl_pct = if pos.entry_price > 0.0 {
                        (pos.mark_price - pos.entry_price) / pos.entry_price
                            * 100.0
                            * if pos.side.to_lowercase() == "sell" {
                                -1.0
                            } else {
                                1.0
                            }
                    } else {
                        0.0
                    };
                    let margin_used = if pos.leverage > 0 {
                        notional / pos.leverage as f64
                    } else {
                        notional
                    };
                    let liq_price = pos.liquidation_price.unwrap_or(0.0);
                    let liq_dist_pct = liq_distance_pct(pos.mark_price, liq_price);

                    // Cosmetic markers only — the canonical health and warning
                    // list come from classify_health above.
                    let warn_liq = liq_dist_pct < cfg.min_liq_distance_pct;
                    let warn_notional = notional > cfg.max_position_notional;

                    total_notional += notional;
                    total_pnl += pnl;
                    total_margin += margin_used;

                    let pnl_sign = if pnl >= 0.0 { "+" } else { "" };
                    let pnl_pct_sign = if pnl_pct >= 0.0 { "+" } else { "" };

                    // Notional line — warn if oversized
                    let notional_str = if warn_notional {
                        format!("${:.2}  {}", notional, "[WARN — oversized]".yellow())
                            .to_string()
                    } else {
                        format!("${:.2}", notional)
                    };

                    // Liq distance line — warn if too close
                    let liq_str = if warn_liq {
                        format!("{:.2}%  {}", liq_dist_pct, "[DANGER — below 10%]".red())
                            .to_string()
                    } else {
                        format!("{:.2}%", liq_dist_pct)
                    };

                    println!();
                    println!(
                        "  {}  {} {}x",
                        pos.symbol.bold(),
                        pos.side.to_uppercase(),
                        pos.leverage
                    );
                    println!("    Size:     {}    Notional: {}", pos.size, notional_str);
                    println!(
                        "    PnL:      {}{:.4} USDT  ({}{:.2}%)",
                        pnl_sign, pnl, pnl_pct_sign, pnl_pct
                    );
                    println!("    Margin:   ${:.2}    Liq dist: {}", margin_used, liq_str);
                }

                println!();
                println!("  {}", "─".repeat(55));
                println!("  TOTALS");
                println!("  Notional:      ${:.2}", total_notional);
                let total_sign = if total_pnl >= 0.0 { "+" } else { "" };
                println!("  Total PnL:     {}{:.4} USDT", total_sign, total_pnl);
                println!("  Total Margin:  ${:.2}", total_margin);
            }

            // Warnings list
            if !warnings.is_empty() {
                println!();
                println!("  WARNINGS");
                for w in &warnings {
                    println!("  {} {}", "[!]".yellow().bold(), w.yellow());
                }
            }

            // Status banner
            println!();
            println!("  {}", "━".repeat(55));
            match health {
                HealthLevel::Healthy => println!("  STATUS: {}", "HEALTHY".green().bold()),
                HealthLevel::Watch => println!("  STATUS: {}", "WATCH".yellow().bold()),
                HealthLevel::Danger => println!("  STATUS: {}", "DANGER".red().bold()),
            }
            println!("  {}", "━".repeat(55));
            println!();

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> PortfolioConfig {
        // Defaults: max_margin_utilization=80, min_liq_distance_pct=10,
        // max_position_notional=5000.
        PortfolioConfig::default()
    }

    fn pos(symbol: &str, mark: f64, liq: Option<f64>, notional: Option<f64>) -> Position {
        Position {
            symbol: symbol.into(),
            side: "long".into(),
            position_side: "long".into(),
            size: 1.0,
            entry_price: mark,
            mark_price: mark,
            pnl: Some(0.0),
            leverage: 10,
            liquidation_price: liq,
            margin_type: None,
            unrealized_pnl: Some(0.0),
            notional,
        }
    }

    // ─── liq_distance_pct ────────────────────────────────────────────────

    #[test]
    fn liq_distance_zero_or_missing_means_no_signal() {
        // Missing liq → 100% (no risk signal). Same for missing/zero mark.
        assert_eq!(liq_distance_pct(100.0, 0.0), 100.0);
        assert_eq!(liq_distance_pct(0.0, 50.0), 100.0);
    }

    #[test]
    fn liq_distance_long_position_basics() {
        // Mark=100, liq=90 → 10% away
        assert!((liq_distance_pct(100.0, 90.0) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn liq_distance_returns_absolute_value_for_short() {
        // Short positions can have liq above mark; the distance is still
        // reported as a positive percentage.
        assert!((liq_distance_pct(100.0, 110.0) - 10.0).abs() < 1e-9);
    }

    // ─── utilization_pct ────────────────────────────────────────────────

    #[test]
    fn utilization_zero_balance_is_zero() {
        // Don't divide by zero — agents shouldn't see Inf/NaN in their report.
        assert_eq!(utilization_pct(0.0, 100.0), 0.0);
    }

    #[test]
    fn utilization_normal_case() {
        assert!((utilization_pct(1000.0, 800.0) - 80.0).abs() < 1e-9);
    }

    // ─── classify_health: empty / all healthy ───────────────────────────

    #[test]
    fn empty_positions_with_no_balance_is_healthy() {
        let (h, w) = classify_health(0.0, 0.0, &[], &cfg());
        assert_eq!(h, HealthLevel::Healthy);
        assert!(w.is_empty());
    }

    #[test]
    fn empty_positions_with_low_utilization_is_healthy() {
        // 50% utilization, no positions → HEALTHY
        let (h, w) = classify_health(1000.0, 500.0, &[], &cfg());
        assert_eq!(h, HealthLevel::Healthy);
        assert!(w.is_empty());
    }

    #[test]
    fn safe_position_with_low_utilization_is_healthy() {
        // Mark=100, liq=80 → 20% away (safe). Notional=$1000 (under $5000).
        let p = pos("BTCUSDT", 100.0, Some(80.0), Some(1000.0));
        let (h, w) = classify_health(1000.0, 100.0, &[p], &cfg());
        assert_eq!(h, HealthLevel::Healthy);
        assert!(w.is_empty());
    }

    // ─── classify_health: utilization boundary ───────────────────────────

    #[test]
    fn utilization_at_exact_threshold_is_healthy() {
        // 80% exactly — strict `>` means HEALTHY.
        let (h, _) = classify_health(1000.0, 800.0, &[], &cfg());
        assert_eq!(h, HealthLevel::Healthy);
    }

    #[test]
    fn utilization_just_over_threshold_is_watch() {
        // 80.0001% — over.
        let (h, w) = classify_health(1_000_000.0, 800_001.0, &[], &cfg());
        assert_eq!(h, HealthLevel::Watch);
        assert!(w.iter().any(|m| m.contains("Margin utilization")));
    }

    // ─── classify_health: liq-distance boundary ─────────────────────────

    #[test]
    fn liq_distance_at_exact_threshold_is_healthy() {
        // mark=100, liq=90 → exactly 10%. Strict `<` means HEALTHY.
        let p = pos("BTCUSDT", 100.0, Some(90.0), Some(100.0));
        let (h, _) = classify_health(1000.0, 0.0, &[p], &cfg());
        assert_eq!(h, HealthLevel::Healthy);
    }

    #[test]
    fn liq_distance_just_below_threshold_is_danger() {
        // mark=100, liq=91 → 9% — under threshold.
        let p = pos("BTCUSDT", 100.0, Some(91.0), Some(100.0));
        let (h, w) = classify_health(1000.0, 0.0, &[p], &cfg());
        assert_eq!(h, HealthLevel::Danger);
        assert!(w.iter().any(|m| m.contains("liq distance")));
    }

    // ─── classify_health: notional boundary ──────────────────────────────

    #[test]
    fn notional_at_exact_threshold_is_healthy() {
        // Notional == max ($5000). Strict `>` means HEALTHY.
        let p = pos("BTCUSDT", 100.0, Some(80.0), Some(5_000.0));
        let (h, _) = classify_health(10_000.0, 0.0, &[p], &cfg());
        assert_eq!(h, HealthLevel::Healthy);
    }

    #[test]
    fn notional_just_over_threshold_is_watch() {
        let p = pos("BTCUSDT", 100.0, Some(80.0), Some(5_000.01));
        let (h, w) = classify_health(10_000.0, 0.0, &[p], &cfg());
        assert_eq!(h, HealthLevel::Watch);
        assert!(w.iter().any(|m| m.contains("notional")));
    }

    // ─── DANGER overrides WATCH ─────────────────────────────────────────

    #[test]
    fn danger_overrides_watch_when_both_fire() {
        // Oversized AND too close to liq → DANGER, with both warnings present.
        let p = pos("BTCUSDT", 100.0, Some(95.0), Some(10_000.0));
        let (h, w) = classify_health(10_000.0, 0.0, &[p], &cfg());
        assert_eq!(h, HealthLevel::Danger);
        assert_eq!(w.len(), 2);
    }

    // ─── multiple positions aggregate ────────────────────────────────────

    #[test]
    fn multiple_positions_aggregate_warnings_and_take_worst_health() {
        // pos A is healthy; pos B is oversized (WATCH); pos C is too-close-liq (DANGER).
        let a = pos("AAA", 100.0, Some(80.0), Some(1_000.0));
        let b = pos("BBB", 100.0, Some(80.0), Some(7_000.0));
        let c = pos("CCC", 100.0, Some(98.0), Some(1_000.0));
        let (h, w) = classify_health(50_000.0, 0.0, &[a, b, c], &cfg());
        assert_eq!(h, HealthLevel::Danger);
        // Two warnings: BBB notional, CCC liq distance.
        assert_eq!(w.len(), 2);
        assert!(w.iter().any(|m| m.contains("BBB")));
        assert!(w.iter().any(|m| m.contains("CCC")));
        assert!(!w.iter().any(|m| m.contains("AAA")));
    }

    // ─── custom thresholds via PortfolioConfig ──────────────────────────

    #[test]
    fn custom_thresholds_are_respected() {
        let custom = PortfolioConfig {
            max_margin_utilization: 50.0,
            min_liq_distance_pct: 20.0,
            max_position_notional: 100.0,
        };
        // util 60% → WATCH; liq dist 15% → DANGER; notional 200 → WATCH (DANGER wins overall)
        let p = pos("BTCUSDT", 100.0, Some(85.0), Some(200.0));
        let (h, w) = classify_health(1000.0, 600.0, &[p], &custom);
        assert_eq!(h, HealthLevel::Danger);
        assert_eq!(w.len(), 3);
    }

    // ─── HealthLevel ordering sanity ────────────────────────────────────

    #[test]
    fn health_level_ordering() {
        // The classify_health logic relies on `<` and `>` between levels.
        assert!(HealthLevel::Healthy < HealthLevel::Watch);
        assert!(HealthLevel::Watch < HealthLevel::Danger);
        // And worst-of selection uses `if existing < new { existing = new }`.
        assert_eq!(
            std::cmp::max(HealthLevel::Watch, HealthLevel::Healthy),
            HealthLevel::Watch
        );
        assert_eq!(
            std::cmp::max(HealthLevel::Watch, HealthLevel::Danger),
            HealthLevel::Danger
        );
    }
}
