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
        OrderSubcommands::Dca(args) => place_dca(args, settings).await,
    }
}

fn determine_side(buy: bool, sell: bool) -> Result<OrderSide> {
    match (buy, sell) {
        (true, false) => Ok(OrderSide::Buy),
        (false, true) => Ok(OrderSide::Sell),
        (false, false) => Err(TtcError::InvalidOrder(
            "Must specify --buy or --sell".into(),
        )),
        (true, true) => Err(TtcError::InvalidOrder(
            "Cannot specify both --buy and --sell".into(),
        )),
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

async fn place_limit(
    args: OrderLimitArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
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
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

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

    info!(
        "Placing limit order: {} {} {} @ {}",
        side, args.quantity, args.symbol, args.price
    );

    let result = client
        .place_limit_order(&args.exchange, params, credentials)
        .await?;

    printer.success(&format!(
        "Limit order placed: {} {} {} @ {}",
        result.side, result.quantity, result.symbol, args.price
    ));
    printer.print(&result);

    Ok(())
}

async fn place_market(
    args: OrderMarketArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
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
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

    let params = MarketOrderParams {
        symbol: args.symbol.clone(),
        side,
        quantity: args.quantity,
        position_side: convert_position_side(args.position_side),
        reduce_only: if args.reduce_only { Some(true) } else { None },
        client_order_id: args.client_order_id,
    };

    info!(
        "Placing market order: {} {} {}",
        side, args.quantity, args.symbol
    );

    let result = client
        .place_market_order(&args.exchange, params, credentials)
        .await?;

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
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

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

    info!(
        "Placing stop order: {} {} {} @ stop {}",
        side, args.quantity, args.symbol, args.stop_price
    );

    let result = client
        .place_stop_order(&args.exchange, params, credentials)
        .await?;

    printer.success(&format!(
        "Stop order placed: {} {} {} @ {}",
        result.side, result.quantity, result.symbol, args.stop_price
    ));
    printer.print(&result);

    Ok(())
}

async fn place_take_profit(
    args: OrderTakeProfitArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
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
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

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

    info!(
        "Placing take profit order: {} {} {} @ {}",
        side, args.quantity, args.symbol, args.tp_price
    );

    let result = client
        .place_stop_order(&args.exchange, params, credentials)
        .await?;

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
        return Err(TtcError::InvalidOrder(
            "Either --order-id or --client-order-id is required".into(),
        ));
    }

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would cancel order on {} for {}",
            args.exchange, args.symbol
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

    let params = CancelOrderParams {
        symbol: args.symbol.clone(),
        order_id: args.order_id,
        client_order_id: args.client_order_id,
    };

    info!("Canceling order on {}", args.exchange);

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

async fn cancel_all(
    args: OrderCancelAllArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    let printer = Printer::new(format);

    if settings.trading.dry_run {
        printer.dry_run(&format!("Would cancel all orders on {}", args.exchange));
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

    info!("Canceling all orders on {}", args.exchange);

    let result = client
        .cancel_all_orders(&args.exchange, args.symbol.as_deref(), credentials)
        .await?;

    printer.success(&result.message);

    Ok(())
}

async fn list_open(args: OrderOpenArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
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

    printer.info(&format!(
        "Found {} open order(s) on {}",
        orders.len(),
        args.exchange
    ));
    printer.print_list(&orders);

    Ok(())
}

async fn place_dca(args: OrderDcaArgs, settings: &AppConfig) -> Result<()> {
    if !args.buy && !args.sell {
        return Err(TtcError::InvalidOrder(
            "Must specify --buy or --sell".into(),
        ));
    }

    let side = if args.buy {
        OrderSide::Buy
    } else {
        OrderSide::Sell
    };
    let side_label = if args.buy { "BUY" } else { "SELL" };
    let min_usd = settings.trading.min_usd_entry;

    // Calculate levels from total amount / min entry
    let levels = ((args.amount / min_usd).floor() as u32).max(1);
    let level_usd = args.amount / levels as f64;

    let client = Client::new(settings)?;
    let credentials = get_credentials(
        &args.exchange,
        args.api_key,
        args.api_secret,
        args.passphrase,
        settings,
    )?;

    // Fetch current price if not provided
    let base_price = if let Some(p) = args.start_price {
        p
    } else {
        let ticker_params = GetTickersParams {
            symbol: Some(args.symbol.clone()),
        };
        let tickers = client
            .get_tickers(&args.exchange, ticker_params, credentials.clone())
            .await?;
        tickers
            .iter()
            .find(|t| t.symbol.to_uppercase() == args.symbol.to_uppercase())
            .ok_or_else(|| TtcError::InvalidOrder(format!("Symbol {} not found", args.symbol)))?
            .last_price
    };

    if base_price <= 0.0 {
        return Err(TtcError::InvalidOrder(
            "Got zero price from exchange".into(),
        ));
    }

    let price_factor = 10f64.powi(args.price_decimals as i32);
    let qty_factor = 10f64.powi(args.qty_decimals as i32);
    let step = args.distance / 100.0;

    println!();
    println!(
        "  DCA Ladder — {} {} on {}",
        args.symbol, side_label, args.exchange
    );
    println!(
        "  Amount:  ${:.2} total  |  Levels: {}  |  Per level: ${:.2}",
        args.amount, levels, level_usd
    );
    println!(
        "  Base:    ${:.4}  |  Step: {:.2}% per level  |  Min entry: ${:.2}",
        base_price, args.distance, min_usd
    );
    println!("  ─────────────────────────────────────────────────────");
    println!();

    if settings.trading.dry_run {
        for n in 0..levels {
            let price = if args.buy {
                (base_price * (1.0 - step).powi(n as i32) * price_factor).floor() / price_factor
            } else {
                (base_price * (1.0 + step).powi(n as i32) * price_factor).floor() / price_factor
            };
            let qty = (level_usd / price * qty_factor).floor() / qty_factor;
            println!(
                "  DRY-RUN  Level {}/{}  Price: ${:.4}  Qty: {}  Cost: ~${:.2}",
                n + 1,
                levels,
                price,
                qty,
                level_usd
            );
        }
        println!();
        return Ok(());
    }

    let mut placed = 0u32;
    let mut total_cost = 0.0_f64;
    let mut total_qty = 0.0_f64;

    for n in 0..levels {
        let price = if args.buy {
            (base_price * (1.0 - step).powi(n as i32) * price_factor).floor() / price_factor
        } else {
            (base_price * (1.0 + step).powi(n as i32) * price_factor).floor() / price_factor
        };

        let qty = (level_usd / price * qty_factor).floor() / qty_factor;
        if qty <= 0.0 {
            println!(
                "  [{}/{}]  SKIP — qty rounds to zero at ${:.4}",
                n + 1,
                levels,
                price
            );
            continue;
        }

        let params = LimitOrderParams {
            symbol: args.symbol.clone(),
            side,
            quantity: qty,
            price,
            position_side: None,
            time_in_force: None,
            reduce_only: None,
            take_profit_price: None,
            stop_loss_price: None,
            client_order_id: Some(format!(
                "dca-{}-{}-{}",
                args.symbol.to_lowercase(),
                n + 1,
                levels
            )),
        };

        match client
            .place_limit_order(&args.exchange, params, credentials.clone())
            .await
        {
            Ok(order) => {
                placed += 1;
                total_cost += level_usd;
                total_qty += qty;
                println!(
                    "  [{}/{}]  Price: ${:.4}  Qty: {}  Cost: ~${:.2}  Order: {}",
                    n + 1,
                    levels,
                    price,
                    qty,
                    level_usd,
                    order.order_id
                );
            }
            Err(e) => {
                println!("  [{}/{}]  ERROR: {} — skipping level", n + 1, levels, e);
            }
        }
    }

    println!();
    println!("  ─────────────────────────────────────────────────────");
    println!("  DCA complete — {}/{} levels placed", placed, levels);
    println!(
        "  Total allocated: ${:.2}  |  Total qty: {}  |  Avg price: ${:.4}",
        total_cost,
        total_qty,
        if total_qty > 0.0 {
            total_cost / total_qty
        } else {
            0.0
        }
    );
    println!();

    Ok(())
}
