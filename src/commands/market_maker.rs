//! Market-maker loop command.
//!
//! Each round:
//!   1. Fetch best bid/ask
//!   2. Place a limit entry order at best bid (buy mode) or best ask (sell mode)
//!   3. Poll until the entry fills or a timeout elapses (cancel on timeout)
//!   4. Place a limit exit order at entry ± spread (reduce_only)
//!   5. Poll until the exit fills
//!   6. Report gross and net PnL for the round, then loop
//!
//! commission from [market-maker] limit_order_commission in config.toml is used
//! only for net PnL display — the exchange enforces its own fees independently.

use crate::api::Client;
use crate::cli::MarketMakerArgs;
use crate::commands::common::get_credentials;
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use crate::models::*;
use colored::Colorize;
use std::time::{Duration, Instant};

pub async fn execute(args: MarketMakerArgs, settings: &AppConfig) -> Result<()> {
    if !args.buy && !args.sell {
        return Err(TtcError::InvalidOrder(
            "Must specify --buy or --sell".into(),
        ));
    }

    let side_label = if args.buy { "BUY" } else { "SELL" };
    let commission = settings.market_maker.limit_order_commission;
    let max_rounds = if args.rounds == 0 { u32::MAX } else { args.rounds };
    let price_factor = 10f64.powi(args.price_decimals as i32);

    // Percentage spread takes precedence; absolute spread is the fallback
    let use_pct = args.spread == 0.0 || args.spread_pct > 0.0;
    let spread_label = if use_pct {
        format!("{:.3}%", args.spread_pct)
    } else {
        format!("{}", args.spread)
    };

    println!();
    println!("{}", "━".repeat(60));
    println!(
        "  MARKET MAKER  {}  {}  qty: {}  spread: {}",
        args.symbol, side_label, args.quantity, spread_label
    );
    println!(
        "  Exchange: {}  Commission: {:.4}%/side  Min spread: {:.3}%  Poll: {}ms  Timeout: {}s",
        args.exchange,
        commission * 100.0,
        settings.market_maker.min_spread * 100.0,
        args.poll_ms,
        args.timeout_secs,
    );
    println!("{}", "━".repeat(60));
    println!();

    let client = Client::new(settings)?;
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

    let mut total_net_pnl = 0.0_f64;
    let mut rounds_completed = 0u32;

    for round in 1..=max_rounds {
        let round_label = if args.rounds == 0 {
            format!("R{}", round)
        } else {
            format!("R{}/{}", round, args.rounds)
        };

        // ── 1. Fetch best bid/ask ─────────────────────────────────────────
        let bba = match client
            .get_best_bid_ask(
                &args.exchange,
                GetBestBidAskParams {
                    symbol: args.symbol.clone(),
                },
                credentials.clone(),
            )
            .await
        {
            Ok(b) => b,
            Err(e) => {
                eprintln!("  [{}] BBA fetch failed: {} — skipping round", round_label, e);
                tokio::time::sleep(Duration::from_millis(args.poll_ms)).await;
                continue;
            }
        };

        let entry_price_raw: f64 = if args.buy {
            bba.best_bid.price.parse().unwrap_or(0.0)
        } else {
            bba.best_ask.price.parse().unwrap_or(0.0)
        };

        if entry_price_raw <= 0.0 {
            eprintln!("  [{}] Got zero price from BBA — skipping round", round_label);
            tokio::time::sleep(Duration::from_millis(args.poll_ms)).await;
            continue;
        }

        // Round entry price to configured decimals
        let entry_price = (entry_price_raw * price_factor).floor() / price_factor;

        // Compute effective spread for this round
        let raw_spread = if use_pct {
            entry_price * args.spread_pct / 100.0
        } else {
            args.spread
        };
        // Enforce config min_spread floor (stored as fraction, e.g. 0.001 = 0.1%)
        let min_spread_abs = entry_price * settings.market_maker.min_spread;
        let spread = ((raw_spread.max(min_spread_abs)) * price_factor).round() / price_factor;

        // ── 2. Place entry limit order ────────────────────────────────────
        let entry_side = if args.buy { OrderSide::Buy } else { OrderSide::Sell };
        let entry_params = LimitOrderParams {
            symbol: args.symbol.clone(),
            side: entry_side,
            quantity: args.quantity,
            price: entry_price,
            position_side: None,
            time_in_force: Some(TimeInForce::GoodTillCancel),
            reduce_only: None,
            client_order_id: Some(format!("mm-entry-{}-{}", args.symbol.to_lowercase(), round)),
            take_profit_price: None,
            stop_loss_price: None,
        };

        if settings.trading.dry_run {
            let exit_price = if args.buy {
                entry_price + spread
            } else {
                entry_price - spread
            };
            let gross = spread * args.quantity;
            let cost = 2.0 * commission * entry_price * args.quantity;
            println!(
                "  DRY-RUN [{}]  Entry {} @ ${:.prec$}  →  Exit @ ${:.prec$}  spread: ${:.prec$}  gross: ${:.4}  net: ${:.4}",
                round_label, side_label, entry_price, exit_price, spread, gross, gross - cost,
                prec = args.price_decimals as usize
            );
            continue;
        }

        let entry_order = match client
            .place_limit_order(&args.exchange, entry_params, credentials.clone())
            .await
        {
            Ok(o) => o,
            Err(e) => {
                eprintln!("  [{}] Entry order failed: {} — skipping round", round_label, e);
                tokio::time::sleep(Duration::from_millis(args.poll_ms)).await;
                continue;
            }
        };

        println!(
            "  [{}]  {}  Entry @ ${:.prec$}  qty: {}  order: {}",
            round_label,
            side_label.cyan().bold(),
            entry_price,
            args.quantity,
            entry_order.order_id,
            prec = args.price_decimals as usize
        );

        // ── 3. Poll for entry fill ────────────────────────────────────────
        let filled = poll_until_filled(
            &client,
            &args.exchange,
            &args.symbol,
            &entry_order.order_id,
            args.poll_ms,
            args.timeout_secs,
            credentials.clone(),
        )
        .await;

        if !filled {
            // Timed out — cancel the entry order
            eprintln!(
                "  [{}] Entry not filled within {}s — cancelling",
                round_label, args.timeout_secs
            );
            let cancel_params = CancelOrderParams {
                symbol: args.symbol.clone(),
                order_id: Some(entry_order.order_id.clone()),
                client_order_id: None,
            };
            let _ = client
                .cancel_order(&args.exchange, cancel_params, credentials.clone())
                .await;
            continue;
        }

        println!(
            "  [{}]  {}  Entry filled @ ${:.prec$}",
            round_label,
            "✓".green().bold(),
            entry_price,
            prec = args.price_decimals as usize
        );

        // ── 4. Place exit limit order ─────────────────────────────────────
        let exit_price_raw = if args.buy {
            entry_price + spread
        } else {
            entry_price - spread
        };
        let exit_price = (exit_price_raw * price_factor).round() / price_factor;

        let exit_side = if args.buy { OrderSide::Sell } else { OrderSide::Buy };
        let exit_params = LimitOrderParams {
            symbol: args.symbol.clone(),
            side: exit_side,
            quantity: args.quantity,
            price: exit_price,
            position_side: None,
            time_in_force: Some(TimeInForce::GoodTillCancel),
            reduce_only: Some(true),
            client_order_id: Some(format!("mm-exit-{}-{}", args.symbol.to_lowercase(), round)),
            take_profit_price: None,
            stop_loss_price: None,
        };

        let exit_order = match client
            .place_limit_order(&args.exchange, exit_params, credentials.clone())
            .await
        {
            Ok(o) => o,
            Err(e) => {
                eprintln!(
                    "  [{}] Exit order failed: {} — POSITION MAY BE OPEN, check manually",
                    round_label, e
                );
                break;
            }
        };

        println!(
            "  [{}]  {}  Exit @ ${:.prec$}  order: {}",
            round_label,
            if args.buy { "SELL" } else { "BUY" }.yellow().bold(),
            exit_price,
            exit_order.order_id,
            prec = args.price_decimals as usize
        );

        // ── 5. Poll for exit fill ─────────────────────────────────────────
        let exit_filled = poll_until_filled(
            &client,
            &args.exchange,
            &args.symbol,
            &exit_order.order_id,
            args.poll_ms,
            // Exit order gets 3× longer timeout — we really want it to fill
            args.timeout_secs.saturating_mul(3),
            credentials.clone(),
        )
        .await;

        if !exit_filled {
            eprintln!(
                "  [{}] Exit order not filled — still open (order {}). Stopping loop.",
                round_label, exit_order.order_id
            );
            break;
        }

        // ── 6. PnL report ─────────────────────────────────────────────────
        let gross_pnl = spread * args.quantity;
        let commission_cost = 2.0 * commission * entry_price * args.quantity;
        let net_pnl = gross_pnl - commission_cost;
        total_net_pnl += net_pnl;
        rounds_completed += 1;

        let net_str = if net_pnl >= 0.0 {
            format!("${:.4}", net_pnl).green().to_string()
        } else {
            format!("-${:.4}", net_pnl.abs()).red().to_string()
        };

        println!(
            "  [{}]  {}  gross: ${:.4}  fee: ${:.4}  net: {}  cumulative: ${:.4}",
            round_label,
            "✓ ROUND COMPLETE".green().bold(),
            gross_pnl,
            commission_cost,
            net_str,
            total_net_pnl,
        );
        println!();
    }

    // ── Final summary ─────────────────────────────────────────────────────────
    println!("{}", "━".repeat(60));
    println!(
        "  Market maker stopped  |  {} rounds  |  Total net PnL: {}",
        rounds_completed,
        if total_net_pnl >= 0.0 {
            format!("${:.4}", total_net_pnl).green().to_string()
        } else {
            format!("-${:.4}", total_net_pnl.abs()).red().to_string()
        }
    );
    println!("{}", "━".repeat(60));
    println!();

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Poll open orders every `poll_ms` ms until `order_id` is no longer present
/// (meaning it was filled or cancelled). Returns `true` if the order disappeared
/// within the timeout, `false` if it was still open when the timeout elapsed.
/// If `timeout_secs` is 0, waits forever.
async fn poll_until_filled(
    client: &Client,
    exchange: &str,
    symbol: &str,
    order_id: &str,
    poll_ms: u64,
    timeout_secs: u64,
    credentials: ExchangeCredentials,
) -> bool {
    let started = Instant::now();
    let timeout = if timeout_secs == 0 {
        Duration::MAX
    } else {
        Duration::from_secs(timeout_secs)
    };

    loop {
        tokio::time::sleep(Duration::from_millis(poll_ms)).await;

        if started.elapsed() >= timeout {
            return false;
        }

        match client
            .get_orders(exchange, Some(symbol), credentials.clone())
            .await
        {
            Ok(orders) => {
                let still_open = orders.iter().any(|o| o.order_id == order_id);
                if !still_open {
                    return true;
                }
            }
            Err(_) => {
                // API hiccup — keep polling rather than aborting
                continue;
            }
        }
    }
}
