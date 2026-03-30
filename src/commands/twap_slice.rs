//! Single atomic TWAP slice — designed for agent-controlled /loop execution.
//!
//! Places exactly one market order for a fixed USD notional amount.
//! Prints a single result line the agent can read and react to.
//! Use this with Claude Code's /loop instead of the full `twap` command
//! when you want the agent to retain visibility and control on every tick.

use crate::api::Client;
use crate::cli::TwapSliceArgs;
use crate::commands::common::get_credentials;
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use crate::models::*;

pub async fn execute(args: TwapSliceArgs, settings: &AppConfig) -> Result<()> {
    if !args.buy && !args.sell {
        return Err(TtcError::InvalidOrder("Must specify --buy or --sell".into()));
    }

    let side = if args.buy { "BUY" } else { "SELL" };
    let label = args.label.as_deref().unwrap_or("1/1");

    if settings.trading.dry_run {
        println!("  DRY-RUN twap-slice [{}] {} {} — would place ${:.2} notional market order",
            label, args.symbol, side, args.amount);
        return Ok(());
    }

    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

    // Fetch current price
    let ticker_params = GetTickersParams { symbol: Some(args.symbol.clone()) };
    let tickers = client.get_tickers(&args.exchange, ticker_params, credentials.clone()).await?;

    let ticker = tickers.iter()
        .find(|t| t.symbol.to_uppercase() == args.symbol.to_uppercase())
        .ok_or_else(|| TtcError::InvalidOrder(format!("Symbol {} not found on {}", args.symbol, args.exchange)))?;

    let price = ticker.last_price;
    if price <= 0.0 {
        return Err(TtcError::InvalidOrder(format!("Got zero price for {} on {}", args.symbol, args.exchange)));
    }

    // Calculate quantity rounded to lot size
    let factor = 10f64.powi(args.decimals as i32);
    let qty = (args.amount / price * factor).floor() / factor;
    if qty <= 0.0 {
        return Err(TtcError::InvalidOrder(format!(
            "Calculated qty is zero — ${:.2} / ${:.4} rounds to 0 at {} decimals. Increase --amount or --decimals.",
            args.amount, price, args.decimals
        )));
    }

    // Place market order
    let order_side = if args.buy { OrderSide::Buy } else { OrderSide::Sell };
    let params = MarketOrderParams {
        symbol: args.symbol.clone(),
        side: order_side,
        quantity: qty,
        position_side: None,
        reduce_only: None,
        client_order_id: None,
    };

    let order = client.place_market_order(&args.exchange, params, credentials).await?;

    // Single-line output — easy for the agent to read
    println!(
        "  SLICE [{}]  {} {}  Price: ${:.4}  Qty: {}  Cost: ~${:.2}  Order: {}",
        label, args.symbol, side, price, qty, args.amount, order.order_id
    );

    Ok(())
}
