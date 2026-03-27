//! Market data command implementations

use crate::api::Client;
use crate::cli::{MarketCommands, MarketSubcommands, MarketTickersArgs, MarketBestBidAskArgs};
use crate::commands::common::get_credentials;
use crate::config::AppConfig;
use crate::error::Result;
use crate::models::*;
use crate::output::{OutputFormat, Printer};
use tracing::info;

pub async fn execute(cmd: MarketCommands, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    match cmd.command {
        MarketSubcommands::Tickers(args) => get_tickers(args, settings, format).await,
        MarketSubcommands::BestBidAsk(args) => get_best_bid_ask(args, settings, format).await,
    }
}

async fn get_tickers(args: MarketTickersArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

    info!("Fetching tickers from {}", args.exchange);

    let params = GetTickersParams {
        symbol: args.symbol.clone(),
    };

    let tickers = client.get_tickers(&args.exchange, params, credentials).await?;

    if tickers.is_empty() {
        printer.info(&format!("No tickers found on {}", args.exchange));
    } else {
        printer.info(&format!("Found {} ticker(s) on {}", tickers.len(), args.exchange));
    }

    printer.print_list(&tickers);

    Ok(())
}

async fn get_best_bid_ask(args: MarketBestBidAskArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;
    let credentials = get_credentials(&args.exchange, args.api_key, args.api_secret, args.passphrase, settings)?;

    info!("Fetching best bid/ask for {} on {}", args.symbol, args.exchange);

    let params = GetBestBidAskParams {
        symbol: args.symbol.clone(),
    };

    let result = client.get_best_bid_ask(&args.exchange, params, credentials).await?;

    let bid: f64 = result.best_bid.price.parse().unwrap_or(0.0);
    let ask: f64 = result.best_ask.price.parse().unwrap_or(0.0);
    let spread = ask - bid;

    printer.success(&format!("Best Bid/Ask for {} on {}", args.symbol, args.exchange));
    println!("  Bid: {} @ ${:.2}", result.best_bid.quantity, bid);
    println!("  Ask: {} @ ${:.2}", result.best_ask.quantity, ask);
    println!("  Spread: ${:.4} ({:.4}%)", spread, if bid > 0.0 { (spread / bid) * 100.0 } else { 0.0 });

    Ok(())
}
