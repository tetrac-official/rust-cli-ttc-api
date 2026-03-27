//! Order command implementations

use crate::api::Client;
use crate::cli::*;
use crate::commands::common::{convert_position_side, get_credentials};
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use crate::models::*;
use crate::output::{OutputFormat, Printer};
use tracing::info;

pub async fn execute(cmd: OrderCommands, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    match cmd.command {
        OrderSubcommands::Limit(args) => place_limit(args, settings, format).await,
        OrderSubcommands::Market(args) => place_market(args, settings, format).await,
        OrderSubcommands::Stop(args) => place_stop(args, settings, format).await,
        OrderSubcommands::TakeProfit(args) => place_take_profit(args, settings, format).await,
        OrderSubcommands::Cancel(args) => cancel(args, settings, format).await,
        OrderSubcommands::CancelAll(args) => cancel_all(args, settings, format).await,
        OrderSubcommands::Open(args) => list_open(args, settings, format).await,
    }
}

fn determine_side(buy: bool, sell: bool) -> Result<OrderSide> {
    match (buy, sell) {
        (true, false) => Ok(OrderSide::Buy),
        (false, true) => Ok(OrderSide::Sell),
        (false, false) => Err(TtcError::InvalidOrder("Must specify --buy or --sell".into())),
        (true, true) => Err(TtcError::InvalidOrder("Cannot specify both --buy and --sell".into())),
    }
}

fn convert_tif(tif: TimeInForceArg) -> TimeInForce {
    match tif {
        TimeInForceArg::Gtc => TimeInForce::GoodTillCancel,
        TimeInForceArg::Ioc => TimeInForce::ImmediateOrCancel,
        TimeInForceArg::Fok => TimeInForce::FillOrKill,
        TimeInForceArg::Postonly => TimeInForce::PostOnly,
    }
}

fn convert_trigger(trigger: TriggerTypeArg) -> TriggerType {
    match trigger {
        TriggerTypeArg::Last => TriggerType::ByLastPrice,
        TriggerTypeArg::Mark => TriggerType::ByMarkPrice,
        TriggerTypeArg::Index => TriggerType::ByIndexPrice,
    }
}

async fn place_limit(args: OrderLimitArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let side = determine_side(args.buy, args.sell)?;

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would place limit order: {} {} {} @ {}",
            side, args.quantity, args.symbol, args.price
        ));
        return Ok(());
    }

    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

    let params = LimitOrderParams {
        symbol: args.symbol.clone(),
        side,
        quantity: args.quantity,
        price: args.price,
        position_side: convert_position_side(args.position_side),
        time_in_force: Some(convert_tif(args.time_in_force)),
        reduce_only: if args.reduce_only { Some(true) } else { None },
        client_order_id: args.client_order_id,
        take_profit_price: None,
        stop_loss_price: None,
    };

    info!("Placing limit order: {} {} {} @ {}", side, args.quantity, args.symbol, args.price);

    let result = client.place_limit_order(&args.exchange, params, credentials).await?;

    printer.success(&format!(
        "Limit order placed: {} {} {} @ {}",
        result.side, result.quantity, result.symbol, args.price
    ));
    printer.print(&result);

    Ok(())
}

async fn place_market(args: OrderMarketArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let side = determine_side(args.buy, args.sell)?;

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would place market order: {} {} {}",
            side, args.quantity, args.symbol
        ));
        return Ok(());
    }

    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

    let params = MarketOrderParams {
        symbol: args.symbol.clone(),
        side,
        quantity: args.quantity,
        position_side: convert_position_side(args.position_side),
        reduce_only: if args.reduce_only { Some(true) } else { None },
        client_order_id: args.client_order_id,
    };

    info!("Placing market order: {} {} {}", side, args.quantity, args.symbol);

    let result = client.place_market_order(&args.exchange, params, credentials).await?;

    printer.success(&format!(
        "Market order placed: {} {} {}",
        result.side, result.quantity, result.symbol
    ));
    printer.print(&result);

    Ok(())
}

async fn place_stop(args: OrderStopArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let side = determine_side(args.buy, args.sell)?;

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would place stop order: {} {} {} @ stop {}",
            side, args.quantity, args.symbol, args.stop_price
        ));
        return Ok(());
    }

    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

    let params = StopOrderParams {
        symbol: args.symbol.clone(),
        side,
        quantity: args.quantity,
        stop_price: args.stop_price,
        position_side: convert_position_side(args.position_side),
        trigger_type: Some(convert_trigger(args.trigger)),
        reduce_only: None,
        client_order_id: None,
        price: None,
        close_position: None,
    };

    info!("Placing stop order: {} {} {} @ stop {}", side, args.quantity, args.symbol, args.stop_price);

    let result = client.place_stop_order(&args.exchange, params, credentials).await?;

    printer.success(&format!(
        "Stop order placed: {} {} {} @ {}",
        result.side, result.quantity, result.symbol, args.stop_price
    ));
    printer.print(&result);

    Ok(())
}

async fn place_take_profit(args: OrderTakeProfitArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let side = determine_side(args.buy, args.sell)?;

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would place take profit order: {} {} {} @ {}",
            side, args.quantity, args.symbol, args.tp_price
        ));
        return Ok(());
    }

    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

    // Take profit is essentially a stop order for the opposite direction
    let params = StopOrderParams {
        symbol: args.symbol.clone(),
        side,
        quantity: args.quantity,
        stop_price: args.tp_price,
        position_side: convert_position_side(args.position_side),
        trigger_type: Some(TriggerType::ByMarkPrice),
        reduce_only: Some(true),
        client_order_id: None,
        price: None,
        close_position: None,
    };

    info!("Placing take profit order: {} {} {} @ {}", side, args.quantity, args.symbol, args.tp_price);

    let result = client.place_stop_order(&args.exchange, params, credentials).await?;

    printer.success(&format!(
        "Take profit order placed: {} {} {} @ {}",
        result.side, result.quantity, result.symbol, args.tp_price
    ));
    printer.print(&result);

    Ok(())
}

async fn cancel(args: OrderCancelArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);

    if args.order_id.is_none() && args.client_order_id.is_none() {
        return Err(TtcError::InvalidOrder("Either --order-id or --client-order-id is required".into()));
    }

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would cancel order on {} for {}",
            args.exchange, args.symbol
        ));
        return Ok(());
    }

    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

    let params = CancelOrderParams {
        symbol: args.symbol.clone(),
        order_id: args.order_id,
        client_order_id: args.client_order_id,
    };

    info!("Canceling order on {}", args.exchange);

    let result = client.cancel_order(&args.exchange, params, credentials).await?;

    printer.success(&format!("Order canceled: {} on {}", result.order_id, args.exchange));
    println!("Status: {}", result.status);

    Ok(())
}

async fn cancel_all(args: OrderCancelAllArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would cancel all orders on {}",
            args.exchange
        ));
        return Ok(());
    }

    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

    info!("Canceling all orders on {}", args.exchange);

    let results = client.cancel_all_orders(&args.exchange, args.symbol.as_deref(), credentials).await?;

    printer.success(&format!("Canceled {} order(s)", results.len()));
    
    for result in &results {
        println!("  - {}: {}", result.order_id, result.status);
    }

    Ok(())
}

async fn list_open(args: OrderOpenArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

    info!("Fetching open orders from {}", args.exchange);

    let orders = client.get_orders(&args.exchange, args.symbol.as_deref(), credentials).await?;

    printer.info(&format!("Found {} open order(s) on {}", orders.len(), args.exchange));
    printer.print_list(&orders);

    Ok(())
}
