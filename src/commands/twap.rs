//! TWAP (Time-Weighted Average Price) position builder

use crate::api::Client;
use crate::cli::TwapArgs;
use crate::commands::common::get_credentials;
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use crate::models::*;
use std::time::Duration;

pub async fn execute(args: TwapArgs, settings: &AppConfig) -> Result<()> {
    if !args.buy && !args.sell {
        return Err(TtcError::InvalidOrder("Must specify --buy or --sell".into()));
    }

    let side = if args.buy { "buy" } else { "sell" };
    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

    // Calculate slices and interval
    let slices = args.slices.unwrap_or_else(|| {
        ((args.hours * 60.0) / args.interval as f64).ceil() as u32
    }).max(1);
    let interval_secs = if args.slices.is_some() {
        // User overrode slices — recalculate interval from hours
        ((args.hours * 3600.0) / slices as f64) as u64
    } else {
        args.interval * 60
    };
    let slice_usd = args.budget / slices as f64;

    println!();
    println!("  TWAP — {} {} on {}", args.symbol, side.to_uppercase(), args.exchange);
    println!("  Budget:   ${:.2} over {:.1}h", args.budget, args.hours);
    println!("  Slices:   {} orders × ${:.2} each", slices, slice_usd);
    println!("  Interval: {} min between orders", interval_secs / 60);
    println!("  ─────────────────────────────────────────────────────");
    println!();

    if settings.trading.dry_run {
        println!("  DRY-RUN: Would place {} market {} orders of ${:.2} each on {} over {:.1}h",
            slices, side, slice_usd, args.exchange, args.hours);
        return Ok(());
    }

    let mut total_spent = 0.0_f64;
    let mut total_qty = 0.0_f64;
    let mut filled_slices = 0u32;

    for i in 1..=slices {
        // Fetch current price via tickers
        let ticker_params = GetTickersParams { symbol: Some(args.symbol.clone()) };
        let tickers = client.get_tickers(&args.exchange, ticker_params, credentials.clone()).await?;

        let ticker = tickers.iter()
            .find(|t| t.symbol.to_uppercase() == args.symbol.to_uppercase())
            .ok_or_else(|| TtcError::InvalidOrder(format!("Symbol {} not found on {}", args.symbol, args.exchange)))?;

        let price = ticker.last_price;
        if price <= 0.0 {
            println!("  [{}/{}] WARNING: Got zero price — skipping slice", i, slices);
            continue;
        }

        // Calculate quantity for this slice, rounded to exchange lot size
        let factor = 10f64.powi(args.decimals as i32);
        let qty = (slice_usd / price * factor).floor() / factor;
        if qty <= 0.0 {
            println!("  [{}/{}] WARNING: Calculated qty is zero — slice too small", i, slices);
            continue;
        }

        // Place market order
        let order_side = if args.buy { OrderSide::Buy } else { OrderSide::Sell };
        let params = MarketOrderParams {
            symbol: args.symbol.clone(),
            side: order_side,
            quantity: qty,
            position_side: None,
            reduce_only: None,
            client_order_id: Some(format!("twap-{}-{}-{}", args.symbol.to_lowercase(), i, slices)),
        };

        match client.place_market_order(&args.exchange, params, credentials.clone()).await {
            Ok(order) => {
                total_spent += slice_usd;
                total_qty += qty;
                filled_slices += 1;
                println!(
                    "  [{}/{}]  Price: ${:.4}  Qty: {}  Order: {}  [deployed: ${:.2} / ${:.2}]",
                    i, slices, price, qty, order.order_id, total_spent, args.budget
                );
            }
            Err(e) => {
                println!("  [{}/{}]  ERROR: {} — skipping slice", i, slices, e);
            }
        }

        // Sleep between slices (skip sleep after last one)
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

    println!();
    println!("  ─────────────────────────────────────────────────────");
    println!("  TWAP complete — {} / {} slices filled", filled_slices, slices);
    println!("  Total deployed: ${:.2}  |  Total qty: {}  |  Avg price: ${:.4}",
        total_spent,
        total_qty,
        if total_qty > 0.0 { total_spent / total_qty } else { 0.0 }
    );
    println!();

    Ok(())
}
