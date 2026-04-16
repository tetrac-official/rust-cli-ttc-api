//! Portfolio health report — aggregates balance + all positions into one view.

use crate::api::Client;
use crate::cli::{PortfolioCommands, PortfolioSubcommands};
use crate::commands::common::get_credentials;
use crate::config::AppConfig;
use crate::error::Result;
use crate::output::OutputFormat;
use colored::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HealthLevel {
    Healthy,
    Watch,
    Danger,
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
            let locked = bal.map(|b| b.locked).unwrap_or(0.0);
            let utilization_pct = if total_balance > 0.0 {
                locked / total_balance * 100.0
            } else {
                0.0
            };
            let warn_utilization = utilization_pct > cfg.max_margin_utilization;

            // Compute per-position derived values
            let mut warnings: Vec<String> = vec![];
            let mut health = HealthLevel::Healthy;

            let mut total_notional = 0.0_f64;
            let mut total_pnl = 0.0_f64;
            let mut total_margin = 0.0_f64;

            if warn_utilization {
                warnings.push(format!(
                    "Margin utilization {:.1}% exceeds threshold ({:.1}%)",
                    utilization_pct, cfg.max_margin_utilization
                ));
                if health < HealthLevel::Watch {
                    health = HealthLevel::Watch;
                }
            }

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
                    utilization_pct,
                    format!("[WATCH — threshold {:.0}%]", cfg.max_margin_utilization).yellow()
                );
            } else {
                println!("  Utilization:  {:.1}%", utilization_pct);
            }

            // Positions section
            println!();
            if positions.is_empty() {
                println!("  POSITIONS — none open");
            } else {
                println!("  POSITIONS ({} open)", positions.len());
                println!("  {}", "─".repeat(55));

                for pos in &positions {
                    let pnl = pos.unrealized_pnl;
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
                        pos.notional / pos.leverage as f64
                    } else {
                        pos.notional
                    };
                    let liq_dist_pct = if pos.liquidation_price > 0.0 && pos.mark_price > 0.0 {
                        ((pos.mark_price - pos.liquidation_price) / pos.mark_price * 100.0).abs()
                    } else {
                        100.0
                    };

                    let warn_liq = liq_dist_pct < cfg.min_liq_distance_pct;
                    let warn_notional = pos.notional > cfg.max_position_notional;

                    if warn_liq {
                        if health < HealthLevel::Danger {
                            health = HealthLevel::Danger;
                        }
                        warnings.push(format!(
                            "{} liq distance {:.2}% is below threshold ({:.1}%)",
                            pos.symbol, liq_dist_pct, cfg.min_liq_distance_pct
                        ));
                    }
                    if warn_notional {
                        if health < HealthLevel::Watch {
                            health = HealthLevel::Watch;
                        }
                        warnings.push(format!(
                            "{} notional ${:.2} exceeds max_position_notional (${:.2})",
                            pos.symbol, pos.notional, cfg.max_position_notional
                        ));
                    }

                    total_notional += pos.notional;
                    total_pnl += pnl;
                    total_margin += margin_used;

                    let pnl_sign = if pnl >= 0.0 { "+" } else { "" };
                    let pnl_pct_sign = if pnl_pct >= 0.0 { "+" } else { "" };

                    // Notional line — warn if oversized
                    let notional_str = if warn_notional {
                        format!("${:.2}  {}", pos.notional, "[WARN — oversized]".yellow())
                            .to_string()
                    } else {
                        format!("${:.2}", pos.notional)
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
