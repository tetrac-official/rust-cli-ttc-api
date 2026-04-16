//! Orders command implementations

use crate::api::Client;
use crate::cli::*;
use crate::commands::common::get_credentials;
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use crate::models::*;
use crate::output::{OutputFormat, Printer};
use tracing::info;

pub async fn execute(
    cmd: OrdersCommands,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    match cmd.command {
        OrdersSubcommands::Get(args) => get_orders(args, settings, format).await,
        OrdersSubcommands::CancelAll(args) => cancel_all_orders(args, settings, format).await,
        OrdersSubcommands::Cancel(args) => cancel_order(args, settings, format).await,
    }
}

async fn get_orders(args: OrdersGetArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

    info!("Fetching open orders from {}", args.exchange);

    let orders = client
        .get_orders(&args.exchange, args.symbol.as_deref(), credentials)
        .await?;

    if orders.is_empty() {
        printer.info(&format!("No open orders on {}", args.exchange));
    } else {
        printer.info(&format!(
            "Found {} open order(s) on {}",
            orders.len(),
            args.exchange
        ));
    }

    printer.print_list(&orders);

    Ok(())
}

async fn cancel_all_orders(
    args: OrdersCancelAllArgs,
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
            "Would cancel all orders{} on {}",
            args.symbol
                .as_ref()
                .map(|s| format!(" for {}", s))
                .unwrap_or_default(),
            args.exchange
        ));
        return Ok(());
    }

    info!("Canceling all orders on {}", args.exchange);

    let result = client
        .cancel_all_orders(&args.exchange, args.symbol.as_deref(), credentials)
        .await?;

    printer.success(&format!("{} ({})", result.message, args.exchange));

    Ok(())
}

async fn cancel_order(
    args: OrdersCancelArgs,
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

    if args.order_id.is_none() && args.client_order_id.is_none() {
        return Err(TtcError::InvalidOrder(
            "Either --order-id or --client-order-id is required".into(),
        ));
    }

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would cancel order {} for {}",
            args.order_id
                .as_ref()
                .or(args.client_order_id.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("unknown"),
            args.symbol
        ));
        return Ok(());
    }

    info!("Canceling order on {}", args.exchange);

    let params = CancelOrderParams {
        symbol: args.symbol.clone(),
        order_id: args.order_id,
        client_order_id: args.client_order_id,
    };

    let cancelled = client
        .cancel_order(&args.exchange, params, credentials)
        .await?;

    if cancelled {
        printer.success(&format!("Order cancelled on {}", args.exchange));
    } else {
        printer.info(&format!(
            "Order not found or already cancelled on {}",
            args.exchange
        ));
    }

    Ok(())
}
