//! Risk management command implementations

use crate::api::Client;
use crate::cli::*;
use crate::commands::common::{get_credentials, parse_position_side};
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use crate::models::*;
use crate::output::{OutputFormat, Printer};
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;

// ── Trail-watch progress file ────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
struct TrailWatchProgress {
    symbol: String,
    exchange: String,
    position_side: String,
    active: bool,
    mark_price: f64,
    entry_price: f64,
    peak_price: Option<f64>,
    trail_pct: f64,
    current_stop: Option<f64>,
    stop_order_id: Option<String>,
    unrealized_pnl: f64,
    position_size: f64,
    updated_at: String,
}

fn trail_watch_path(symbol: &str, exchange: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(format!(
        ".trail-watch-{}-{}.json",
        symbol.to_lowercase(),
        exchange.to_lowercase()
    ))
}

fn save_trail_watch_progress(progress: &TrailWatchProgress) {
    let path = trail_watch_path(&progress.symbol, &progress.exchange);
    if let Ok(json) = serde_json::to_string_pretty(progress) {
        let _ = std::fs::write(&path, json);
    }
}

fn remove_trail_watch_progress(symbol: &str, exchange: &str) {
    let path = trail_watch_path(symbol, exchange);
    let _ = std::fs::remove_file(path);
}

pub async fn execute(cmd: RiskCommands, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    match cmd.command {
        RiskSubcommands::Sl(args) => set_stop_loss(args, settings, format).await,
        RiskSubcommands::Tp(args) => set_take_profit(args, settings, format).await,
        RiskSubcommands::Trail(args) => set_trailing_stop(args, settings, format).await,
        RiskSubcommands::TrailWatch(args) => trail_watch(args, settings, format).await,
    }
}

/// Find a matching position for the given symbol and optional side filter
fn find_position(
    positions: &[Position],
    position_side: Option<PositionSideArg>,
) -> Option<&Position> {
    positions.iter().find(|p| {
        p.size > 0.0
            && position_side.is_none_or(|ps| {
                let expected = match ps {
                    PositionSideArg::Long => "long",
                    PositionSideArg::Short => "short",
                    PositionSideArg::Both => "both",
                };
                p.position_side.to_lowercase() == expected
            })
    })
}

/// Determine the order side to close a position (sell for long, buy for short)
fn closing_side(pos_side: PositionSide) -> Result<OrderSide> {
    match pos_side {
        PositionSide::Long => Ok(OrderSide::Sell),
        PositionSide::Short => Ok(OrderSide::Buy),
        PositionSide::Both => Err(TtcError::InvalidPosition(
            "Must specify position side for hedge mode".into(),
        )),
    }
}

async fn set_stop_loss(
    args: RiskStopLossArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would set stop loss at {} for {} on {}",
            args.stop_price, args.symbol, args.exchange
        ));
        return Ok(());
    }

    let positions = client
        .get_positions(&args.exchange, Some(&args.symbol), credentials.clone())
        .await?;

    let position = find_position(&positions, args.position_side)
        .ok_or_else(|| TtcError::PositionNotFound(args.symbol.clone()))?;

    let pos_side = parse_position_side(&position.position_side);
    let stop_side = closing_side(pos_side)?;

    info!(
        "Setting stop loss for {} {} position at {}",
        args.symbol, position.position_side, args.stop_price
    );

    let params = StopOrderParams {
        symbol: args.symbol.clone(),
        side: stop_side,
        quantity: position.size.abs(),
        stop_price: args.stop_price,
        position_side: Some(pos_side),
        trigger_type: Some(TriggerType::ByMarkPrice),
        reduce_only: Some(true),
        client_order_id: None,
        price: None,
        close_position: None,
    };

    let result = client
        .place_stop_order(&args.exchange, params, credentials)
        .await?;

    printer.success(&format!(
        "Stop loss set at {} for {} {} position (qty: {})",
        args.stop_price, args.symbol, position.position_side, position.size
    ));
    printer.print(&result);

    Ok(())
}

async fn set_take_profit(
    args: RiskTakeProfitArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would set take profit at {} for {} on {}",
            args.tp_price, args.symbol, args.exchange
        ));
        return Ok(());
    }

    let positions = client
        .get_positions(&args.exchange, Some(&args.symbol), credentials.clone())
        .await?;

    let position = find_position(&positions, args.position_side)
        .ok_or_else(|| TtcError::PositionNotFound(args.symbol.clone()))?;

    let pos_side = parse_position_side(&position.position_side);
    let tp_side = closing_side(pos_side)?;

    info!(
        "Setting take profit for {} {} position at {}",
        args.symbol, position.position_side, args.tp_price
    );

    let params = StopOrderParams {
        symbol: args.symbol.clone(),
        side: tp_side,
        quantity: position.size.abs(),
        stop_price: args.tp_price,
        position_side: Some(pos_side),
        trigger_type: Some(TriggerType::ByMarkPrice),
        reduce_only: Some(true),
        client_order_id: None,
        price: None,
        close_position: None,
    };

    let result = client
        .place_stop_order(&args.exchange, params, credentials)
        .await?;

    printer.success(&format!(
        "Take profit set at {} for {} {} position (qty: {})",
        args.tp_price, args.symbol, position.position_side, position.size
    ));
    printer.print(&result);

    Ok(())
}

async fn set_trailing_stop(
    args: RiskTrailingStopArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would set trailing stop ({}%) for {} on {}",
            args.distance, args.symbol, args.exchange
        ));
        return Ok(());
    }

    let positions = client
        .get_positions(&args.exchange, Some(&args.symbol), credentials.clone())
        .await?;

    let position = find_position(&positions, args.position_side)
        .ok_or_else(|| TtcError::PositionNotFound(args.symbol.clone()))?;

    let pos_side = parse_position_side(&position.position_side);
    let trail_side = closing_side(pos_side)?;

    // Calculate trailing stop distance as a percentage of mark price
    let mark_price = position.mark_price;
    let trail_distance = mark_price * args.distance / 100.0;

    // Calculate initial stop price based on position direction
    // Long positions: stop below mark price; Short positions: stop above mark price
    let initial_stop = match pos_side {
        PositionSide::Long => mark_price - trail_distance,
        PositionSide::Short => mark_price + trail_distance,
        PositionSide::Both => unreachable!(),
    };

    info!(
        "Setting trailing stop for {} {} position (distance: {}%)",
        args.symbol, position.position_side, args.distance
    );

    // Note: Trailing stop implementation varies by exchange
    // This uses the stop order approach; some exchanges have native trailing stops
    let params = StopOrderParams {
        symbol: args.symbol.clone(),
        side: trail_side,
        quantity: position.size.abs(),
        stop_price: initial_stop,
        position_side: Some(pos_side),
        trigger_type: Some(TriggerType::ByMarkPrice),
        reduce_only: Some(true),
        client_order_id: None,
        price: None,
        close_position: None,
    };

    let result = client
        .place_stop_order(&args.exchange, params, credentials)
        .await?;

    printer.success(&format!(
        "Trailing stop set for {} {} position (trail: {:.2}%, initial stop: {:.4})",
        args.symbol, position.position_side, args.distance, initial_stop
    ));
    printer.print(&result);

    Ok(())
}

async fn trail_watch(
    args: RiskTrailWatchArgs,
    settings: &AppConfig,
    _format: OutputFormat,
) -> Result<()> {
    let client = Client::new(settings)?;
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

    let progress_path = trail_watch_path(&args.symbol, &args.exchange);

    println!();
    println!("  Trail Watch — {} on {}", args.symbol, args.exchange);
    println!("  Trail:    {:.2}%", args.trail_pct);
    println!("  Interval: {}s", args.interval);
    println!("  State:    {}", progress_path.display());
    println!("  Waiting for position to enter profit before activating...");
    println!("  Press Ctrl+C to stop.");
    println!();

    let mut peak: Option<f64> = None;
    let mut current_stop_price: Option<f64> = None;
    // Track current stop order ID so we can cancel it specifically after placing the new one.
    // This is the place-then-cancel pattern: new stop is live before old one is removed,
    // so the position is never unprotected even if a cancel or place call fails.
    let mut current_stop_order_id: Option<String> = None;
    let mut active = false;

    loop {
        let positions = client
            .get_positions(&args.exchange, Some(&args.symbol), credentials.clone())
            .await?;

        let position = find_position(&positions, args.position_side);

        match position {
            None => {
                println!(
                    "  [trail-watch] No open {} position found — stopping.",
                    args.symbol
                );
                break;
            }
            Some(pos) => {
                let mark = pos.mark_price;
                let entry = pos.entry_price;
                let pnl = pos.unrealized_pnl;
                let pos_side = parse_position_side(&pos.position_side);

                if !active {
                    if pnl > 0.0 {
                        active = true;
                        peak = Some(mark);
                        println!(
                            "  [trail-watch] Position entered profit at ${:.4} — activating trail.",
                            mark
                        );
                    } else {
                        let gap = ((entry - mark) / entry * 100.0).abs();
                        println!(
                            "  [trail-watch] Waiting for profit. Mark: ${:.4}  Entry: ${:.4}  PnL: ${:.2}  Gap: {:.2}%",
                            mark, entry, pnl, gap
                        );
                    }

                    // Write progress even while waiting for activation
                    save_trail_watch_progress(&TrailWatchProgress {
                        symbol: args.symbol.clone(),
                        exchange: args.exchange.clone(),
                        position_side: pos.position_side.clone(),
                        active: false,
                        mark_price: mark,
                        entry_price: entry,
                        peak_price: None,
                        trail_pct: args.trail_pct,
                        current_stop: None,
                        stop_order_id: None,
                        unrealized_pnl: pnl,
                        position_size: pos.size,
                        updated_at: chrono::Utc::now().to_rfc3339(),
                    });
                }

                if active {
                    let p = peak.get_or_insert(mark);
                    match pos_side {
                        PositionSide::Long | PositionSide::Both => {
                            if mark > *p {
                                *p = mark;
                            }
                        }
                        PositionSide::Short => {
                            if mark < *p {
                                *p = mark;
                            }
                        }
                    }
                    let p = *p;

                    let new_stop = match pos_side {
                        PositionSide::Long | PositionSide::Both => {
                            p * (1.0 - args.trail_pct / 100.0)
                        }
                        PositionSide::Short => p * (1.0 + args.trail_pct / 100.0),
                    };

                    let should_update = match current_stop_price {
                        None => true,
                        Some(prev) => match pos_side {
                            PositionSide::Long | PositionSide::Both => new_stop > prev,
                            PositionSide::Short => new_stop < prev,
                        },
                    };

                    println!(
                        "  [trail-watch] Mark: ${:.4}  Peak: ${:.4}  Trail stop: ${:.4}  PnL: ${:.2}{}",
                        mark, p, new_stop, pnl,
                        if should_update && current_stop_price.is_some() { "  → updating stop" } else { "" }
                    );

                    if should_update {
                        let stop_side = match pos_side {
                            PositionSide::Long | PositionSide::Both => OrderSide::Sell,
                            PositionSide::Short => OrderSide::Buy,
                        };

                        let stop_params = StopOrderParams {
                            symbol: args.symbol.clone(),
                            side: stop_side,
                            quantity: pos.size.abs(),
                            stop_price: new_stop,
                            position_side: Some(pos_side),
                            trigger_type: Some(TriggerType::ByMarkPrice),
                            reduce_only: Some(true),
                            client_order_id: None,
                            price: None,
                            close_position: None,
                        };

                        // ── PLACE-THEN-CANCEL ────────────────────────────────────────────────
                        // 1. Place the new stop first. If this fails, the old stop stays active
                        //    and the position is never unprotected.
                        match client
                            .place_stop_order(&args.exchange, stop_params, credentials.clone())
                            .await
                        {
                            Ok(new_order) => {
                                let new_id = new_order.order_id.clone();
                                println!(
                                    "  [trail-watch] New stop placed at ${:.4}  (order: {})",
                                    new_stop, new_id
                                );

                                // 2. Cancel the previous stop by ID, now that the new one is live.
                                //    If this fails, both stops exist — harmless; position still protected.
                                if let Some(old_id) = current_stop_order_id.take() {
                                    let cancel_params = CancelOrderParams {
                                        symbol: args.symbol.clone(),
                                        order_id: Some(old_id.clone()),
                                        client_order_id: None,
                                    };
                                    match client.cancel_order(&args.exchange, cancel_params, credentials.clone()).await {
                                        Ok(_)  => println!("  [trail-watch] Old stop cancelled  (order: {})", old_id),
                                        Err(e) => println!("  [trail-watch] Warning: old stop cancel failed ({}): {} — new stop is still active", old_id, e),
                                    }
                                }

                                current_stop_price = Some(new_stop);
                                current_stop_order_id = Some(new_id);
                            }
                            Err(e) => {
                                // New stop failed — old stop (if any) is still in place.
                                println!(
                                    "  [trail-watch] Warning: failed to place new stop at ${:.4}: {}{}",
                                    new_stop, e,
                                    if current_stop_price.is_some() { " — existing stop unchanged" } else { " — no stop active" }
                                );
                            }
                        }
                        // ────────────────────────────────────────────────────────────────────
                    }

                    // Write progress on every active tick
                    save_trail_watch_progress(&TrailWatchProgress {
                        symbol: args.symbol.clone(),
                        exchange: args.exchange.clone(),
                        position_side: pos.position_side.clone(),
                        active: true,
                        mark_price: mark,
                        entry_price: entry,
                        peak_price: peak,
                        trail_pct: args.trail_pct,
                        current_stop: current_stop_price,
                        stop_order_id: current_stop_order_id.clone(),
                        unrealized_pnl: pnl,
                        position_size: pos.size,
                        updated_at: chrono::Utc::now().to_rfc3339(),
                    });
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(args.interval)).await;
    }

    // Clean up progress file when position closes
    remove_trail_watch_progress(&args.symbol, &args.exchange);

    Ok(())
}
