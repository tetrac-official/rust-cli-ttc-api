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

/// Convert a USD notional `amount` to a quantity at `price`, floored to the
/// `decimals` lot-size precision. Floors (not rounds) so the executed
/// notional never exceeds the requested amount — overspending in size could
/// trip leverage limits or available-balance checks.
///
/// Returns `None` when the floored quantity is zero (price too high for the
/// amount at this precision); the caller surfaces that as an error.
///
/// Implementation note: a naive `(amount / price * factor).floor()` loses
/// money on inputs where the division yields a value mathematically equal to
/// an integer but represented in f64 as `n - ε`. e.g. `15 / 50_000 * 10_000`
/// is mathematically `3` but f64 gives `2.9999...`, which floors to `2` and
/// underspends $5 on a $15 trade. We snap to the nearest integer if we're
/// already within `EPSILON` of one before flooring.
pub(crate) fn floor_quantity(amount: f64, price: f64, decimals: u32) -> Option<f64> {
    // Reject NaN, infinities, zero, and negatives. is_finite() catches
    // NaN/inf; the <= 0.0 catches zero and negatives.
    if !price.is_finite() || price <= 0.0 || !amount.is_finite() || amount <= 0.0 {
        return None;
    }
    const EPSILON: f64 = 1e-9;
    let factor = 10f64.powi(decimals as i32);
    let raw = amount / price * factor;
    if !raw.is_finite() {
        return None;
    }
    let nudged = if (raw - raw.round()).abs() < EPSILON {
        raw.round()
    } else {
        raw
    };
    let qty = nudged.floor() / factor;
    if qty > 0.0 {
        Some(qty)
    } else {
        None
    }
}

pub async fn execute(args: TwapSliceArgs, settings: &AppConfig) -> Result<()> {
    if !args.buy && !args.sell {
        return Err(TtcError::InvalidOrder(
            "Must specify --buy or --sell".into(),
        ));
    }

    let side = if args.buy { "BUY" } else { "SELL" };
    let label = args.label.as_deref().unwrap_or("1/1");

    if settings.trading.dry_run {
        println!(
            "  DRY-RUN twap-slice [{}] {} {} — would place ${:.2} notional market order",
            label, args.symbol, side, args.amount
        );
        return Ok(());
    }

    let client = Client::new(settings)?;
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

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
        return Err(TtcError::InvalidOrder(format!(
            "Got zero price for {} on {}",
            args.symbol, args.exchange
        )));
    }

    // Calculate quantity floored to lot size
    let qty = floor_quantity(args.amount, price, args.decimals).ok_or_else(|| {
        TtcError::InvalidOrder(format!(
            "Calculated qty is zero — ${:.2} / ${:.4} rounds to 0 at {} decimals. Increase --amount or --decimals.",
            args.amount, price, args.decimals
        ))
    })?;

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
        client_order_id: None,
    };

    let order = client
        .place_market_order(&args.exchange, params, credentials)
        .await?;

    // Single-line output — easy for the agent to read
    println!(
        "  SLICE [{}]  {} {}  Price: ${:.4}  Qty: {}  Cost: ~${:.2}  Order: {}",
        label, args.symbol, side, price, qty, args.amount, order.order_id
    );

    Ok(())
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

    #[test]
    fn floor_quantity_decimals_0_truncates_down() {
        // $15 at price $2.50, integer lot size → 6 (cost $15.00 exact)
        approx(floor_quantity(15.0, 2.5, 0).unwrap(), 6.0);
        // $15 at price $2.51 with int lots → floor(5.976...) = 5 (cost $12.55,
        // undertraded by $2.45). Floor is intentional — never overspend.
        approx(floor_quantity(15.0, 2.51, 0).unwrap(), 5.0);
    }

    #[test]
    fn floor_quantity_more_decimals_preserves_more_notional() {
        // Same inputs, finer precision → less under-traded slack.
        let q0 = floor_quantity(15.0, 2.51, 0).unwrap(); // 5.0 → cost $12.55
        let q4 = floor_quantity(15.0, 2.51, 4).unwrap(); // 5.9760 → cost $14.99976
        assert!(q4 > q0);
        assert!(q4 * 2.51 <= 15.0, "must never overspend");
        assert!(q4 * 2.51 > q0 * 2.51, "more decimals → closer to budget");
    }

    #[test]
    fn floor_quantity_high_price_low_amount_with_decimals_4() {
        // BTC at $50,000 with $15 budget at 4 decimals → 0.0003 (cost $15.00)
        approx(floor_quantity(15.0, 50_000.0, 4).unwrap(), 0.0003);
    }

    #[test]
    fn floor_quantity_returns_none_when_floor_to_zero() {
        // $15 at $50,000 with int lot size → 0 → None (caller errors out).
        assert!(floor_quantity(15.0, 50_000.0, 0).is_none());
    }

    #[test]
    fn floor_quantity_returns_none_for_zero_or_negative_inputs() {
        assert!(floor_quantity(0.0, 100.0, 4).is_none());
        assert!(floor_quantity(-5.0, 100.0, 4).is_none());
        assert!(floor_quantity(15.0, 0.0, 4).is_none());
        assert!(floor_quantity(15.0, -1.0, 4).is_none());
    }

    #[test]
    fn floor_quantity_returns_none_for_nan_or_infinite_inputs() {
        // NaN / inf must not panic and must surface as an error.
        assert!(floor_quantity(f64::NAN, 100.0, 4).is_none());
        assert!(floor_quantity(15.0, f64::NAN, 4).is_none());
        assert!(floor_quantity(f64::INFINITY, 100.0, 4).is_none());
    }

    #[test]
    fn floor_quantity_handles_fp_imprecision_at_high_prices() {
        // Real bug caught here: 15 / 50_000 * 10_000 in f64 is 2.9999...
        // not 3.0, so naive floor underspends ($10 instead of $15). The
        // helper snaps to the nearest integer when within EPSILON.
        approx(floor_quantity(15.0, 50_000.0, 4).unwrap(), 0.0003);
        approx(floor_quantity(15.0, 50_000.0, 4).unwrap() * 50_000.0, 15.0);

        // More sentinels — values that should land exactly on a tick.
        approx(floor_quantity(100.0, 0.5, 0).unwrap(), 200.0);
        approx(floor_quantity(33.33, 33.33, 4).unwrap(), 1.0);
    }

    #[test]
    fn floor_quantity_never_overspends_amount_for_realistic_pairs() {
        // Property-style spot check: across a grid of amount/price pairs,
        // floored qty * price <= amount always holds (the floor invariant).
        for &amount in &[1.0_f64, 10.0, 25.5, 100.0, 1000.0] {
            for &price in &[0.05_f64, 1.0, 2.51, 33.33, 1234.5678, 50_000.0] {
                for &dec in &[0u32, 2, 4, 6] {
                    if let Some(q) = floor_quantity(amount, price, dec) {
                        let spent = q * price;
                        assert!(
                            spent <= amount + 1e-9,
                            "overspend: amount={amount} price={price} decimals={dec} qty={q} spent={spent}"
                        );
                    }
                }
            }
        }
    }
}
