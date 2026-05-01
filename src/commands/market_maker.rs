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
use crate::commands::common::{get_credentials, validate_order_inputs};
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use crate::models::*;
use colored::Colorize;
use std::time::{Duration, Instant};

/// Round a raw price to the configured tick precision in the maker's favor.
///
/// - BUY entry rounds DOWN (floor) — pays less than the live bid.
/// - SELL entry rounds UP (ceil)  — receives more than the live ask.
///
/// Both directions place the order BEHIND the queue (passive maker), which
/// is the right thing for a spread-capture strategy. Symmetrical floor on
/// both sides — the previous behavior — was a bug: floor on a SELL price
/// puts the order BELOW the live ask, undercutting the queue and either
/// matching as a taker or selling for less than the live ask.
pub(crate) fn round_entry_price(raw_price: f64, decimals: u32, is_buy: bool) -> f64 {
    if !raw_price.is_finite() || raw_price <= 0.0 {
        return 0.0;
    }
    let factor = 10f64.powi(decimals as i32);
    let scaled = raw_price * factor;
    if is_buy {
        scaled.floor() / factor
    } else {
        scaled.ceil() / factor
    }
}

/// Compute the per-round spread, rounded to tick precision.
///
/// `requested_spread` is the absolute spread the caller asked for (already
/// derived from --spread or --spread-pct). `min_spread_fraction` is the
/// config floor (e.g. 0.001 = 0.1% of entry_price). The effective spread is
/// the larger of the two, then rounded to the price-tick precision.
pub(crate) fn compute_spread(
    entry_price: f64,
    requested_spread: f64,
    min_spread_fraction: f64,
    decimals: u32,
) -> f64 {
    let min_spread_abs = entry_price * min_spread_fraction;
    let raw = requested_spread.max(min_spread_abs);
    let factor = 10f64.powi(decimals as i32);
    (raw * factor).round() / factor
}

/// Exit price for a market-maker round given entry and spread.
/// Buy entry → sell exit ABOVE entry; Sell entry → buy exit BELOW entry.
pub(crate) fn exit_price(entry_price: f64, spread: f64, is_buy_entry: bool) -> f64 {
    if is_buy_entry {
        entry_price + spread
    } else {
        entry_price - spread
    }
}

pub async fn execute(args: MarketMakerArgs, settings: &AppConfig) -> Result<()> {
    if !args.buy && !args.sell {
        return Err(TtcError::InvalidOrder(
            "Must specify --buy or --sell".into(),
        ));
    }

    validate_order_inputs(&args.symbol, args.quantity)?;

    let side_label = if args.buy { "BUY" } else { "SELL" };
    let commission = settings.market_maker.limit_order_commission;
    // --rounds 0 means "until Ctrl-C" in real runs. In dry-run that would
    // produce an infinite tight loop with no fill polling and no useful
    // signal — cap to 1 so `market-maker --dry-run` previews exactly one
    // round and exits cleanly. Users who want a multi-round preview can
    // pass --rounds N explicitly.
    let max_rounds = match (args.rounds, settings.trading.dry_run) {
        (0, true) => 1,
        (0, false) => u32::MAX,
        (n, _) => n,
    };
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
        // Dry-run must work without network — an agent verifying its plan
        // shouldn't need a live BBA call. On fetch failure in dry-run, fall
        // back to a placeholder so the user/agent still sees the shape of
        // the operation (placeholder price = 1.0).
        let entry_price_raw: f64 = match client
            .get_best_bid_ask(
                &args.exchange,
                GetBestBidAskParams {
                    symbol: args.symbol.clone(),
                },
                credentials.clone(),
            )
            .await
        {
            Ok(bba) => {
                let p: f64 = if args.buy {
                    bba.best_bid.price.parse().unwrap_or(0.0)
                } else {
                    bba.best_ask.price.parse().unwrap_or(0.0)
                };
                if p <= 0.0 && !settings.trading.dry_run {
                    eprintln!(
                        "  [{}] Got zero price from BBA — skipping round",
                        round_label
                    );
                    tokio::time::sleep(Duration::from_millis(args.poll_ms)).await;
                    continue;
                }
                if p <= 0.0 {
                    1.0
                } else {
                    p
                }
            }
            Err(e) if settings.trading.dry_run => {
                eprintln!(
                    "  [{}] BBA unavailable ({}) — using placeholder price 1.0 for dry-run",
                    round_label, e
                );
                1.0
            }
            Err(e) => {
                eprintln!(
                    "  [{}] BBA fetch failed: {} — skipping round",
                    round_label, e
                );
                tokio::time::sleep(Duration::from_millis(args.poll_ms)).await;
                continue;
            }
        };

        // Round entry price in the maker's favor (floor for BUY, ceil for SELL).
        let entry_price = round_entry_price(entry_price_raw, args.price_decimals, args.buy);

        // Compute effective spread for this round, enforcing the config floor.
        let raw_spread = if use_pct {
            entry_price * args.spread_pct / 100.0
        } else {
            args.spread
        };
        let spread = compute_spread(
            entry_price,
            raw_spread,
            settings.market_maker.min_spread,
            args.price_decimals,
        );

        // ── 2. Place entry limit order ────────────────────────────────────
        let entry_side = if args.buy {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };
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
            let exit_price = exit_price(entry_price, spread, args.buy);
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
                eprintln!(
                    "  [{}] Entry order failed: {} — skipping round",
                    round_label, e
                );
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
        let exit_price_raw = exit_price(entry_price, spread, args.buy);
        let exit_price = (exit_price_raw * price_factor).round() / price_factor;

        let exit_side = if args.buy {
            OrderSide::Sell
        } else {
            OrderSide::Buy
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!(
            (a - b).abs() < 1e-9,
            "expected {b}, got {a} (delta {})",
            (a - b).abs()
        );
    }

    // ─── round_entry_price ──────────────────────────────────────────────────

    #[test]
    fn buy_entry_floors_to_below_or_equal_live_bid() {
        // Floor → entry ≤ live bid (passive, behind the queue, pays less).
        approx(round_entry_price(1.1234, 4, true), 1.1234);
        approx(round_entry_price(1.1234, 3, true), 1.123);
        approx(round_entry_price(1.1239, 3, true), 1.123);
        approx(round_entry_price(1.1234, 0, true), 1.0);
    }

    #[test]
    fn sell_entry_ceils_to_above_or_equal_live_ask() {
        // CEIL → entry ≥ live ask (passive, behind the queue, receives more).
        // Previously this was floor — a bug that undercut the ask queue.
        approx(round_entry_price(1.1235, 4, false), 1.1235);
        approx(round_entry_price(1.1235, 3, false), 1.124); // was 1.123 before fix
        approx(round_entry_price(1.1231, 3, false), 1.124);
        approx(round_entry_price(1.1235, 0, false), 2.0); // was 1.0 before fix
    }

    #[test]
    fn entry_price_at_exact_tick_is_unchanged_either_side() {
        // 1.1230 with decimals=3 is already on a tick — both directions
        // return it unchanged.
        approx(round_entry_price(1.123, 3, true), 1.123);
        approx(round_entry_price(1.123, 3, false), 1.123);
    }

    #[test]
    fn entry_rounding_is_in_makers_favor_for_realistic_spreads() {
        // Across a realistic price grid, buy entry ≤ raw and sell entry ≥ raw.
        for &raw in &[0.0123_f64, 1.1234, 33.337, 1234.5678, 50_000.5] {
            for &dec in &[0u32, 1, 2, 4, 6] {
                let buy_entry = round_entry_price(raw, dec, true);
                let sell_entry = round_entry_price(raw, dec, false);
                assert!(
                    buy_entry <= raw + 1e-9,
                    "buy must round down: raw={raw} dec={dec} got {buy_entry}"
                );
                assert!(
                    sell_entry >= raw - 1e-9,
                    "sell must round up: raw={raw} dec={dec} got {sell_entry}"
                );
                // And both should be on a valid tick.
                let factor = 10f64.powi(dec as i32);
                let buy_ticks = buy_entry * factor;
                let sell_ticks = sell_entry * factor;
                assert!((buy_ticks - buy_ticks.round()).abs() < 1e-6);
                assert!((sell_ticks - sell_ticks.round()).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn entry_with_zero_or_negative_input_returns_zero() {
        // Defensive — main loop guards against zero, but the helper should
        // never panic.
        approx(round_entry_price(0.0, 4, true), 0.0);
        approx(round_entry_price(-1.0, 4, false), 0.0);
        approx(round_entry_price(f64::NAN, 4, true), 0.0);
    }

    // ─── compute_spread ─────────────────────────────────────────────────────

    #[test]
    fn spread_uses_requested_when_above_min() {
        // entry=100, requested=0.5 (=0.5%), min_fraction=0.001 (=0.1%, =0.1)
        // 0.5 > 0.1 → use 0.5, rounded to 4 decimals = 0.5
        approx(compute_spread(100.0, 0.5, 0.001, 4), 0.5);
    }

    #[test]
    fn spread_floors_to_min_when_requested_below_min() {
        // entry=100, requested=0.05 (=0.05%), min_fraction=0.001 (=0.1%, =0.1)
        // 0.05 < 0.1 → use 0.1, rounded to 4 decimals = 0.1
        approx(compute_spread(100.0, 0.05, 0.001, 4), 0.1);
    }

    #[test]
    fn spread_rounds_to_tick() {
        // entry=1.123, requested=0.00123, decimals=3 → round(1.23)/1000 = 0.001
        approx(compute_spread(1.123, 0.00123, 0.0, 3), 0.001);
        // entry=1.123, requested=0.00150, decimals=3 → round(1.50)/1000 = 0.002 (banker's-ish)
        let r = compute_spread(1.123, 0.00150, 0.0, 3);
        assert!(
            (r - 0.001).abs() < 1e-9 || (r - 0.002).abs() < 1e-9,
            "got {r}"
        );
    }

    #[test]
    fn spread_zero_min_fraction_does_not_inflate() {
        // min_spread=0 → spread = requested only
        approx(compute_spread(100.0, 0.5, 0.0, 4), 0.5);
        approx(compute_spread(100.0, 0.0, 0.0, 4), 0.0);
    }

    // ─── exit_price ─────────────────────────────────────────────────────────

    #[test]
    fn exit_buy_entry_is_above_entry() {
        // Buy entry → sell exit at entry + spread (we earn the spread).
        approx(exit_price(100.0, 0.5, true), 100.5);
    }

    #[test]
    fn exit_sell_entry_is_below_entry() {
        // Sell entry → buy exit at entry - spread (we earn the spread).
        approx(exit_price(100.0, 0.5, false), 99.5);
    }

    #[test]
    fn round_trip_preserves_spread_as_pnl_per_unit() {
        // For BOTH directions, |exit - entry| = spread.
        // Locks in the symmetry — if either direction breaks, gross PnL drifts.
        let entry = 1.234;
        let spread = 0.005;
        let buy_exit = exit_price(entry, spread, true);
        let sell_exit = exit_price(entry, spread, false);
        approx(buy_exit - entry, spread);
        approx(entry - sell_exit, spread);
    }
}
