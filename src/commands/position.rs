//! Position command implementations

use crate::api::Client;
use crate::cli::*;
use crate::commands::common::{convert_position_side, get_credentials};
use crate::config::AppConfig;
use crate::error::Result;
use crate::models::*;
use crate::output::{OutputFormat, Printer};
use tracing::info;

pub async fn execute(
    cmd: PositionCommands,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    match cmd.command {
        PositionSubcommands::Get(args) => get_positions(args, settings, format).await,
        PositionSubcommands::Pnl(args) => pnl_breakdown(args, settings).await,
        PositionSubcommands::Close(args) => close_position(args, settings, format).await,
        PositionSubcommands::CloseAll(args) => close_all_positions(args, settings, format).await,
    }
}

async fn pnl_breakdown(args: PositionGetArgs, settings: &AppConfig) -> Result<()> {
    let client = Client::new(settings)?;
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

    info!("Fetching positions from {}", args.exchange);

    let positions = client
        .get_positions(&args.exchange, args.symbol.as_deref(), credentials)
        .await?;

    if positions.is_empty() {
        println!("  No open positions on {}", args.exchange);
        return Ok(());
    }

    println!();
    for pos in &positions {
        let pnl = pos.effective_pnl();
        let notional = pos.notional.unwrap_or(0.0);
        let margin_mode = pos.margin_type.as_deref().unwrap_or("n/a");
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

        let liq_price = pos.liquidation_price.unwrap_or(0.0);
        let distance_to_liq = if liq_price > 0.0 && pos.mark_price > 0.0 {
            ((pos.mark_price - liq_price) / pos.mark_price * 100.0).abs()
        } else {
            0.0
        };

        let margin_used = if pos.leverage > 0 {
            notional / pos.leverage as f64
        } else {
            notional
        };

        let pnl_sign = if pnl >= 0.0 { "+" } else { "" };
        let pnl_pct_sign = if pnl_pct >= 0.0 { "+" } else { "" };

        println!(
            "  ── {} {} {}x ─────────────────────────────────",
            pos.symbol,
            pos.side.to_uppercase(),
            pos.leverage
        );
        println!(
            "  Size:          {} units  (${:.2} notional)",
            pos.size, notional
        );
        println!("  Entry price:   ${:.4}", pos.entry_price);
        println!(
            "  Mark price:    ${:.4}  ({}{:.2}% from entry)",
            pos.mark_price, pnl_pct_sign, pnl_pct
        );
        println!(
            "  Unrealized PnL: {}{:.4} USDT  ({}{:.2}%)",
            pnl_sign, pnl, pnl_sign, pnl_pct
        );
        println!(
            "  Margin used:   ${:.2}  ({}x leverage, {} mode)",
            margin_used, pos.leverage, margin_mode
        );
        if let Some(lp) = pos.liquidation_price {
            println!(
                "  Liquidation:   ${:.4}  ({:.2}% away)",
                lp, distance_to_liq
            );
        } else {
            println!("  Liquidation:   n/a");
        }
        println!();
    }

    if positions.len() > 1 {
        let total_pnl: f64 = positions.iter().map(|p| p.effective_pnl()).sum();
        let total_notional: f64 = positions.iter().filter_map(|p| p.notional).sum();
        let pnl_sign = if total_pnl >= 0.0 { "+" } else { "" };
        println!("  ── TOTAL ──────────────────────────────────────");
        println!("  Positions:     {}", positions.len());
        println!("  Total notional: ${:.2}", total_notional);
        println!("  Total PnL:     {}{:.4} USDT", pnl_sign, total_pnl);
        println!();
    }

    Ok(())
}

async fn get_positions(
    args: PositionGetArgs,
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

    info!("Fetching positions from {}", args.exchange);

    let positions = client
        .get_positions(&args.exchange, args.symbol.as_deref(), credentials)
        .await?;

    if positions.is_empty() {
        printer.info(&format!("No open positions on {}", args.exchange));
    } else {
        let total_pnl: f64 = positions.iter().map(|p| p.effective_pnl()).sum();

        printer.info(&format!(
            "Found {} position(s) on {} (Total PnL: {:.4})",
            positions.len(),
            args.exchange,
            total_pnl
        ));
    }

    printer.print_list(&positions);

    Ok(())
}

async fn close_position(
    args: PositionCloseArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    let printer = Printer::new(format);

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would close position {} on {}",
            args.symbol, args.exchange
        ));
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

    info!("Closing position {} on {}", args.symbol, args.exchange);

    let params = ClosePositionParams {
        symbol: args.symbol.clone(),
        position_side: convert_position_side(args.position_side),
        quantity: args.quantity,
    };

    let result = client
        .close_position(&args.exchange, params, credentials)
        .await?;

    printer.success(&format!(
        "Position closed: {} {} on {}",
        result.side, result.symbol, args.exchange
    ));
    printer.print(&result);

    Ok(())
}

async fn close_all_positions(
    args: PositionCloseAllArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    let printer = Printer::new(format);

    if settings.trading.dry_run {
        printer.dry_run(&format!("Would close all positions on {}", args.exchange));
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

    info!("Fetching all positions to close on {}", args.exchange);

    let positions = client
        .get_positions(&args.exchange, None, credentials.clone())
        .await?;

    if positions.is_empty() {
        printer.info(&format!("No open positions to close on {}", args.exchange));
        return Ok(());
    }

    printer.info(&format!("Closing {} position(s)...", positions.len()));

    let mut closed = 0;
    let mut failed = 0;

    for pos in &positions {
        if pos.size == 0.0 {
            continue;
        }

        let position_side = match pos.position_side.as_str() {
            "long" => Some(PositionSide::Long),
            "short" => Some(PositionSide::Short),
            _ => Some(PositionSide::Both),
        };

        let params = ClosePositionParams {
            symbol: pos.symbol.clone(),
            position_side,
            quantity: None,
        };

        match client
            .close_position(&args.exchange, params, credentials.clone())
            .await
        {
            Ok(_) => {
                closed += 1;
                println!("  Closed {} {} ({})", pos.side, pos.symbol, pos.size);
            }
            Err(e) => {
                failed += 1;
                eprintln!("  Failed to close {} {}: {}", pos.side, pos.symbol, e);
            }
        }
    }

    printer.success(&format!("Closed {} position(s), {} failed", closed, failed));

    Ok(())
}
