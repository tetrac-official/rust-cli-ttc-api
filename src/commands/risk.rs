//! Risk management command implementations

use crate::api::Client;
use crate::cli::*;
use crate::commands::common::{get_credentials, parse_position_side};
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use crate::models::*;
use crate::output::{OutputFormat, Printer};
use tracing::info;

pub async fn execute(cmd: RiskCommands, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    match cmd.command {
        RiskSubcommands::Sl(args) => set_stop_loss(args, settings, format).await,
        RiskSubcommands::Tp(args) => set_take_profit(args, settings, format).await,
        RiskSubcommands::Trail(args) => set_trailing_stop(args, settings, format).await,
    }
}

/// Find a matching position for the given symbol and optional side filter
fn find_position(
    positions: &[Position],
    position_side: Option<PositionSideArg>,
) -> Option<&Position> {
    positions.iter().find(|p| {
        p.size > 0.0 && position_side.is_none_or(|ps| {
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
        PositionSide::Both => Err(TtcError::InvalidPosition("Must specify position side for hedge mode".into())),
    }
}

async fn set_stop_loss(args: RiskStopLossArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

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

    let result = client.place_stop_order(&args.exchange, params, credentials).await?;

    printer.success(&format!(
        "Stop loss set at {} for {} {} position (qty: {})",
        args.stop_price, args.symbol, position.position_side, position.size
    ));
    printer.print(&result);

    Ok(())
}

async fn set_take_profit(args: RiskTakeProfitArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

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

    let result = client.place_stop_order(&args.exchange, params, credentials).await?;

    printer.success(&format!(
        "Take profit set at {} for {} {} position (qty: {})",
        args.tp_price, args.symbol, position.position_side, position.size
    ));
    printer.print(&result);

    Ok(())
}

async fn set_trailing_stop(args: RiskTrailingStopArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

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

    let result = client.place_stop_order(&args.exchange, params, credentials).await?;

    printer.success(&format!(
        "Trailing stop set for {} {} position (trail: {:.2}%, initial stop: {:.2})",
        args.symbol,
        position.position_side,
        args.distance,
        initial_stop
    ));
    printer.print(&result);

    Ok(())
}
