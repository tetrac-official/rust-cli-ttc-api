//! Account command implementations

use crate::api::Client;
use crate::cli::*;
use crate::commands::common::get_credentials;
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use crate::models::*;
use crate::output::{OutputFormat, Printer};
use tracing::info;

pub async fn execute(
    cmd: AccountCommands,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    match cmd.command {
        AccountSubcommands::Balance(args) => get_balance(args, settings, format).await,
        AccountSubcommands::Leverage(args) => set_leverage(args, settings, format).await,
        AccountSubcommands::Margin(args) => set_margin_mode(args, settings, format).await,
        AccountSubcommands::Hedge(args) => set_hedge_mode(args, settings, format).await,
    }
}

async fn get_balance(
    args: AccountBalanceArgs,
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

    info!("Fetching balance from {}", args.exchange);

    let balances = client.get_balance(&args.exchange, credentials).await?;

    let total_available: f64 = balances.iter().map(|b| b.available).sum();
    let total_locked: f64 = balances.iter().filter_map(|b| b.locked).sum();

    printer.info(&format!(
        "Balance on {} - Available: {:.4}, Locked: {:.4}",
        args.exchange, total_available, total_locked
    ));

    printer.print_list(&balances);

    Ok(())
}

async fn set_leverage(
    args: AccountLeverageArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    let printer = Printer::new(format);

    if args.leverage == 0 {
        return Err(TtcError::InvalidOrder(
            "Leverage must be greater than 0".into(),
        ));
    }

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would set leverage to {}x for {} on {}",
            args.leverage, args.symbol, args.exchange
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

    info!(
        "Setting leverage to {}x for {} on {}",
        args.leverage, args.symbol, args.exchange
    );

    let params = SetLeverageParams {
        symbol: args.symbol.clone(),
        leverage: args.leverage,
    };

    let result = client
        .set_leverage(&args.exchange, params, credentials)
        .await?;

    printer.success(&format!(
        "Leverage set to {}x for {} on {}",
        result.leverage, result.symbol, args.exchange
    ));

    if let Some(max) = result.max_leverage {
        printer.info(&format!("Maximum leverage allowed: {}x", max));
    }

    Ok(())
}

async fn set_margin_mode(
    args: AccountMarginArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    let printer = Printer::new(format);

    let margin_mode = match args.mode {
        MarginModeArg::Isolated => MarginMode::Isolated,
        MarginModeArg::Cross => MarginMode::Cross,
    };

    if settings.trading.dry_run {
        printer.dry_run(&format!(
            "Would set margin mode to {} on {}",
            margin_mode, args.exchange
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

    info!(
        "Setting margin mode to {} on {}",
        margin_mode, args.exchange
    );

    let params = SetMarginModeParams {
        margin_mode,
        symbol: args.symbol,
    };

    let result = client
        .set_margin_mode(&args.exchange, params, credentials)
        .await?;

    printer.success(&format!(
        "Margin mode set to {} on {}",
        result.margin_mode, args.exchange
    ));

    if let Some(symbol) = result.symbol {
        printer.info(&format!("Applied to symbol: {}", symbol));
    }

    Ok(())
}

async fn set_hedge_mode(
    args: AccountHedgeArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    let printer = Printer::new(format);

    if !args.enable && !args.disable {
        return Err(TtcError::InvalidOrder(
            "Must specify --enable or --disable".into(),
        ));
    }

    let enabled = if args.disable { false } else { args.enable };
    let mode_str = if enabled {
        "hedge mode"
    } else {
        "one-way mode"
    };

    if settings.trading.dry_run {
        printer.dry_run(&format!("Would set {} on {}", mode_str, args.exchange));
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

    info!("Setting {} on {}", mode_str, args.exchange);

    let result = client
        .set_hedge_mode(&args.exchange, enabled, credentials)
        .await?;

    printer.success(&format!(
        "{} enabled on {}",
        if result.hedge_mode {
            "Hedge mode"
        } else {
            "One-way mode"
        },
        args.exchange
    ));

    Ok(())
}
