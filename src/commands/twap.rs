//! TWAP (Time-Weighted Average Price) position builder

use crate::api::Client;
use crate::cli::TwapArgs;
use crate::commands::common::get_credentials;
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use crate::models::*;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

// ── State file ───────────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TwapState {
    symbol: String,
    exchange: String,
    side: String,
    budget: f64,
    hours: f64,
    slices: u32,
    interval_secs: u64,
    slice_usd: f64,
    decimals: u32,
    leverage: Option<u32>,
    filled_slices: u32,
    total_spent: f64,
    total_qty: f64,
    completed_indices: Vec<u32>, // 1-based slice numbers already filled
}

fn state_path(symbol: &str, exchange: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(format!(
        ".twap-{}-{}.json",
        symbol.to_lowercase(),
        exchange.to_lowercase()
    ))
}

fn save_state(state: &TwapState) {
    let path = state_path(&state.symbol, &state.exchange);
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = fs::write(&path, json);
    }
}

fn load_state(symbol: &str, exchange: &str) -> Option<TwapState> {
    let path = state_path(symbol, exchange);
    let json = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&json).ok()
}

fn delete_state(symbol: &str, exchange: &str) {
    let path = state_path(symbol, exchange);
    let _ = fs::remove_file(&path);
}

// ── Execute ──────────────────────────────────────────────────────────────────

pub async fn execute(args: TwapArgs, settings: &AppConfig) -> Result<()> {
    if !args.buy && !args.sell {
        return Err(TtcError::InvalidOrder(
            "Must specify --buy or --sell".into(),
        ));
    }

    let side = if args.buy { "buy" } else { "sell" };
    let client = Client::new(settings)?;
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

    // ── Resume or fresh start ────────────────────────────────────────────────

    let (
        slices,
        interval_secs,
        slice_usd,
        mut total_spent,
        mut total_qty,
        mut filled_slices,
        completed_indices,
    ) = if args.resume {
        match load_state(&args.symbol, &args.exchange) {
            Some(state) => {
                // Validate the resumed state matches current args
                if state.side != side {
                    return Err(TtcError::InvalidOrder(format!(
                            "Saved state is for {} but current args say {}. Delete ~/.twap-{}-{}.json to start fresh.",
                            state.side, side, args.symbol.to_lowercase(), args.exchange.to_lowercase()
                        )));
                }
                println!();
                println!(
                    "  RESUMING TWAP — {} {} on {}",
                    state.symbol,
                    state.side.to_uppercase(),
                    state.exchange
                );
                println!(
                    "  Progress:  {}/{} slices already filled  (${:.2} deployed)",
                    state.filled_slices, state.slices, state.total_spent
                );
                println!(
                    "  Remaining: {} slices × ${:.2} each",
                    state.slices - state.filled_slices,
                    state.slice_usd
                );
                println!("  ─────────────────────────────────────────────────────");
                println!();
                (
                    state.slices,
                    state.interval_secs,
                    state.slice_usd,
                    state.total_spent,
                    state.total_qty,
                    state.filled_slices,
                    state.completed_indices,
                )
            }
            None => {
                return Err(TtcError::InvalidOrder(format!(
                    "No saved TWAP state found for {} on {}. Run without --resume to start fresh.",
                    args.symbol, args.exchange
                )));
            }
        }
    } else {
        // Fresh run — calculate plan
        // Slice count priority:
        //   1. --slices explicit override
        //   2. Auto: floor(budget / min_usd_entry) from config
        // Interval is always recalculated from hours / slices
        let min_usd = settings.trading.min_usd_entry;
        let slices = args
            .slices
            .unwrap_or_else(|| (args.budget / min_usd).floor() as u32)
            .max(1);
        let interval_secs = ((args.hours * 3600.0) / slices as f64) as u64;
        let slice_usd = args.budget / slices as f64;

        println!();
        println!(
            "  TWAP — {} {} on {}",
            args.symbol,
            side.to_uppercase(),
            args.exchange
        );
        println!(
            "  Budget:   ${:.2} notional over {:.1}h",
            args.budget, args.hours
        );
        println!(
            "  Slices:   {} orders × ${:.2} each  (min entry: ${:.2})",
            slices, slice_usd, min_usd
        );
        println!(
            "  Interval: {}m {}s between orders",
            interval_secs / 60,
            interval_secs % 60
        );
        if let Some(lev) = args.leverage {
            println!(
                "  Leverage: {}x  (margin required: ${:.2})",
                lev,
                args.budget / lev as f64
            );
        }
        println!(
            "  State:    ~/.twap-{}-{}.json",
            args.symbol.to_lowercase(),
            args.exchange.to_lowercase()
        );
        println!("  ─────────────────────────────────────────────────────");
        println!();

        if settings.trading.dry_run {
            println!("  DRY-RUN: Would place {} market {} orders of ${:.2} notional each on {} over {:.1}h",
                    slices, side, slice_usd, args.exchange, args.hours);
            if let Some(lev) = args.leverage {
                println!(
                    "  DRY-RUN: Margin required: ${:.2}  (${:.2} / {}x)",
                    args.budget / lev as f64,
                    args.budget,
                    lev
                );
            }
            return Ok(());
        }

        // Set leverage on exchange before first slice if requested
        if let Some(lev) = args.leverage {
            let lev_params = SetLeverageParams {
                symbol: args.symbol.clone(),
                leverage: lev,
            };
            match client
                .set_leverage(&args.exchange, lev_params, credentials.clone())
                .await
            {
                Ok(_) => println!("  Leverage set to {}x on {}", lev, args.exchange),
                Err(e) => println!(
                    "  WARNING: Could not set leverage: {} — continuing anyway",
                    e
                ),
            }
            println!();
        }

        // Write initial state so even if slice 1 fails we have a record
        let init_state = TwapState {
            symbol: args.symbol.clone(),
            exchange: args.exchange.clone(),
            side: side.to_string(),
            budget: args.budget,
            hours: args.hours,
            slices,
            interval_secs,
            slice_usd,
            decimals: args.decimals,
            leverage: args.leverage,
            filled_slices: 0,
            total_spent: 0.0,
            total_qty: 0.0,
            completed_indices: vec![],
        };
        save_state(&init_state);

        (
            slices,
            interval_secs,
            slice_usd,
            0.0_f64,
            0.0_f64,
            0u32,
            vec![],
        )
    };

    // ── Slice loop ───────────────────────────────────────────────────────────

    for i in 1..=slices {
        // Skip slices already completed in a previous run
        if completed_indices.contains(&i) {
            println!(
                "  [{}/{}]  SKIPPED (already filled in previous run)",
                i, slices
            );
            continue;
        }

        // Fetch current price
        let ticker_params = GetTickersParams {
            symbol: Some(args.symbol.clone()),
        };
        let tickers = client
            .get_tickers(&args.exchange, ticker_params, credentials.clone())
            .await?;

        let ticker = tickers
            .iter()
            .find(|t| t.symbol.to_uppercase() == args.symbol.to_uppercase())
            .ok_or_else(|| {
                TtcError::InvalidOrder(format!(
                    "Symbol {} not found on {}",
                    args.symbol, args.exchange
                ))
            })?;

        let price = ticker.last_price;
        if price <= 0.0 {
            println!(
                "  [{}/{}] WARNING: Got zero price — skipping slice",
                i, slices
            );
            continue;
        }

        // Calculate quantity rounded to lot size
        let factor = 10f64.powi(args.decimals as i32);
        let qty = (slice_usd / price * factor).floor() / factor;
        if qty <= 0.0 {
            println!(
                "  [{}/{}] WARNING: Calculated qty is zero — slice too small",
                i, slices
            );
            continue;
        }

        // Place market order
        let order_side = if args.buy {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };
        let params = MarketOrderParams {
            symbol: args.symbol.clone(),
            side: order_side,
            quantity: qty,
            position_side: None,
            reduce_only: None,
            client_order_id: Some(format!(
                "twap-{}-{}-{}",
                args.symbol.to_lowercase(),
                i,
                slices
            )),
        };

        match client
            .place_market_order(&args.exchange, params, credentials.clone())
            .await
        {
            Ok(order) => {
                total_spent += slice_usd;
                total_qty += qty;
                filled_slices += 1;

                println!(
                    "  [{}/{}]  Price: ${:.4}  Qty: {}  Order: {}  [deployed: ${:.2} / ${:.2}]",
                    i, slices, price, qty, order.order_id, total_spent, args.budget
                );

                // Persist state after every successful fill
                let state = TwapState {
                    symbol: args.symbol.clone(),
                    exchange: args.exchange.clone(),
                    side: side.to_string(),
                    budget: args.budget,
                    hours: args.hours,
                    slices,
                    interval_secs,
                    slice_usd,
                    decimals: args.decimals,
                    leverage: args.leverage,
                    filled_slices,
                    total_spent,
                    total_qty,
                    completed_indices: {
                        let mut c = completed_indices.clone();
                        c.push(i);
                        c
                    },
                };
                save_state(&state);
            }
            Err(e) => {
                println!("  [{}/{}]  ERROR: {} — skipping slice", i, slices, e);
            }
        }

        // Sleep between slices (skip after last)
        if i < slices {
            let mins = interval_secs / 60;
            let secs = interval_secs % 60;
            if mins > 0 {
                println!("        Next order in {}m {}s...", mins, secs);
            } else {
                println!("        Next order in {}s...", interval_secs);
            }
            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
        }
    }

    // ── Summary ──────────────────────────────────────────────────────────────

    println!();
    println!("  ─────────────────────────────────────────────────────");
    println!(
        "  TWAP complete — {} / {} slices filled",
        filled_slices, slices
    );
    println!(
        "  Total deployed: ${:.2}  |  Total qty: {}  |  Avg price: ${:.4}",
        total_spent,
        total_qty,
        if total_qty > 0.0 {
            total_spent / total_qty
        } else {
            0.0
        }
    );
    println!();

    // Clean up state file on successful completion
    if filled_slices == slices {
        delete_state(&args.symbol, &args.exchange);
        println!("  State file removed (all slices filled).");
        println!();
    } else {
        let state_file = state_path(&args.symbol, &args.exchange);
        println!(
            "  {} slices were skipped or errored.",
            slices - filled_slices
        );
        println!("  State saved — resume with: skill-trading twap ... --resume");
        println!("  State file: {}", state_file.display());
        println!();
    }

    Ok(())
}
