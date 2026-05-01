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
                let pnl = pos.effective_pnl();
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

#[cfg(test)]
mod tests {
    //! Trail-watch progress file lifecycle.
    //!
    //! Sandboxes $HOME per test so files don't end up in the user's real home.

    use super::*;
    use crate::commands::common::TEST_ENV_LOCK;
    use uuid::Uuid;

    struct SandboxedHome {
        path: PathBuf,
        prev_home: Option<String>,
        _guard: std::sync::MutexGuard<'static, ()>,
    }

    impl SandboxedHome {
        fn new() -> Self {
            let guard = TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
            let prev_home = std::env::var("HOME").ok();
            let path = std::env::temp_dir().join(format!("trail-watch-test-{}", Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            std::env::set_var("HOME", &path);
            Self {
                path,
                prev_home,
                _guard: guard,
            }
        }
    }

    impl Drop for SandboxedHome {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
            match &self.prev_home {
                Some(h) => std::env::set_var("HOME", h),
                None => std::env::remove_var("HOME"),
            }
        }
    }

    fn fixture(symbol: &str, exchange: &str) -> TrailWatchProgress {
        TrailWatchProgress {
            symbol: symbol.into(),
            exchange: exchange.into(),
            position_side: "long".into(),
            active: true,
            mark_price: 31_500.0,
            entry_price: 30_000.0,
            peak_price: Some(31_800.0),
            trail_pct: 2.0,
            current_stop: Some(31_164.0),
            stop_order_id: Some("ord-abc-123".into()),
            unrealized_pnl: 750.0,
            position_size: 0.5,
            updated_at: "2026-04-28T12:00:00Z".into(),
        }
    }

    #[test]
    fn trail_watch_path_is_dotfile_in_home() {
        let _h = SandboxedHome::new();
        let p = trail_watch_path("BTCUSDT", "Orderly");
        let parent = p.parent().unwrap();
        assert_eq!(parent, std::env::var_os("HOME").map(PathBuf::from).unwrap());
        assert_eq!(
            p.file_name().unwrap().to_str().unwrap(),
            ".trail-watch-btcusdt-orderly.json"
        );
    }

    #[test]
    fn trail_watch_path_lowercases_inputs() {
        let _h = SandboxedHome::new();
        assert_eq!(
            trail_watch_path("BtCuSdT", "OrDeRlY"),
            trail_watch_path("btcusdt", "orderly")
        );
    }

    #[test]
    fn save_writes_full_progress_shape_as_valid_json() {
        // TrailWatchProgress is Serialize-only, so we round-trip via Value.
        let _h = SandboxedHome::new();
        let p = fixture("BTCUSDT", "orderly");
        save_trail_watch_progress(&p);

        let raw = std::fs::read_to_string(trail_watch_path(&p.symbol, &p.exchange))
            .expect("file written");
        let v: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");

        // The CLAUDE.md contract: an agent reads this file to inspect state.
        // Lock in the exact field names + types it expects.
        assert_eq!(v["symbol"], "BTCUSDT");
        assert_eq!(v["exchange"], "orderly");
        assert_eq!(v["active"], true);
        assert_eq!(v["mark_price"], 31_500.0);
        assert_eq!(v["entry_price"], 30_000.0);
        assert_eq!(v["peak_price"], 31_800.0);
        assert_eq!(v["trail_pct"], 2.0);
        assert_eq!(v["current_stop"], 31_164.0);
        assert_eq!(v["stop_order_id"], "ord-abc-123");
        assert_eq!(v["unrealized_pnl"], 750.0);
        assert_eq!(v["position_size"], 0.5);
        assert_eq!(v["updated_at"], "2026-04-28T12:00:00Z");
        assert_eq!(v["position_side"], "long");
    }

    #[test]
    fn save_with_none_optionals_writes_null() {
        // Before activation, peak_price / current_stop / stop_order_id are None.
        let _h = SandboxedHome::new();
        let p = TrailWatchProgress {
            symbol: "ETHUSDT".into(),
            exchange: "bybit".into(),
            position_side: "long".into(),
            active: false,
            mark_price: 2000.0,
            entry_price: 2010.0,
            peak_price: None,
            trail_pct: 1.5,
            current_stop: None,
            stop_order_id: None,
            unrealized_pnl: -5.0,
            position_size: 1.0,
            updated_at: "2026-04-28T12:00:00Z".into(),
        };
        save_trail_watch_progress(&p);
        let v: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(trail_watch_path(&p.symbol, &p.exchange)).unwrap(),
        )
        .unwrap();
        assert!(v["peak_price"].is_null());
        assert!(v["current_stop"].is_null());
        assert!(v["stop_order_id"].is_null());
        assert_eq!(v["active"], false);
    }

    #[test]
    fn save_then_remove_clears_the_file() {
        let _h = SandboxedHome::new();
        let p = fixture("DELME", "exch");
        save_trail_watch_progress(&p);
        assert!(trail_watch_path(&p.symbol, &p.exchange).exists());
        remove_trail_watch_progress(&p.symbol, &p.exchange);
        assert!(!trail_watch_path(&p.symbol, &p.exchange).exists());
    }

    #[test]
    fn remove_for_missing_file_is_a_noop() {
        let _h = SandboxedHome::new();
        remove_trail_watch_progress("NEVER", "saved");
    }

    #[test]
    fn save_overwrites_previous_state_in_place() {
        // Each tick replaces the file; verify that a second save replaces
        // the first.
        let _h = SandboxedHome::new();
        let mut p = fixture("OVRWRT", "exch");
        save_trail_watch_progress(&p);

        p.mark_price = 32_000.0;
        p.peak_price = Some(32_500.0);
        p.current_stop = Some(31_850.0);
        save_trail_watch_progress(&p);

        let v: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(trail_watch_path(&p.symbol, &p.exchange)).unwrap(),
        )
        .unwrap();
        assert_eq!(v["mark_price"], 32_000.0);
        assert_eq!(v["peak_price"], 32_500.0);
        assert_eq!(v["current_stop"], 31_850.0);
    }
}
