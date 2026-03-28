//! Market data command implementations

use crate::api::Client;
use crate::cli::*;
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
        MarketSubcommands::HybridTickers(args) => get_hybrid_tickers(args, settings, format).await,
        MarketSubcommands::FundingRates(args) => get_funding_rates(args, settings, format).await,
        MarketSubcommands::OpenInterest(args) => get_open_interest(args, settings, format).await,
        MarketSubcommands::VolumeSnapshot(args) => get_volume_snapshot(args, settings, format).await,
        MarketSubcommands::Scanner(args) => get_scanner(args, settings, format).await,
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
    println!("  Bid: {} @ ${:.4}", result.best_bid.quantity, bid);
    println!("  Ask: {} @ ${:.4}", result.best_ask.quantity, ask);
    println!("  Spread: ${:.4} ({:.4}%)", spread, if bid > 0.0 { (spread / bid) * 100.0 } else { 0.0 });

    Ok(())
}

async fn get_hybrid_tickers(args: MarketHybridTickersArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;

    let market_type = args.market_type.map(|t| match t {
        MarketTypeArg::Spot => "spot",
        MarketTypeArg::Futures => "futures",
    });

    info!("Fetching hybrid tickers");

    let data = client.get_hybrid_tickers(
        market_type,
        args.source.as_deref(),
        args.symbol.as_deref(),
        args.min_volume,
        args.min_price,
        args.max_price,
        args.up,
        args.down,
    ).await?;

    let show_spot = market_type.map(|t| t == "spot").unwrap_or(true);
    let show_futures = market_type.map(|t| t == "futures").unwrap_or(true);

    if show_futures && !data.futures.data.is_empty() {
        printer.info(&format!("Futures: {} markets", data.futures.data.len()));
        for t in &data.futures.data {
            let price: f64 = t.last_price.parse().unwrap_or(0.0);
            let change: f64 = t.price_change_percent.parse().unwrap_or(0.0);
            let vol: f64 = t.quote_volume.parse().unwrap_or(0.0);
            let oi: f64 = t.open_interest.as_deref().unwrap_or("0").parse().unwrap_or(0.0);
            let funding: f64 = t.funding.as_deref().unwrap_or("0").parse().unwrap_or(0.0);
            let sign = if change >= 0.0 { "+" } else { "" };
            println!(
                "  {:<16} ${:<12.4} {:>7}  Vol: ${:.0}  OI: ${:.0}  Fund: {:.4}%  [{}]",
                t.symbol, price, format!("{}{}%", sign, change),
                vol, oi, funding * 100.0, t.source
            );
        }
    }

    if show_spot && !data.spot.data.is_empty() {
        printer.info(&format!("Spot: {} markets", data.spot.data.len()));
        for t in &data.spot.data {
            let price: f64 = t.last_price.parse().unwrap_or(0.0);
            let change: f64 = t.price_change_percent.parse().unwrap_or(0.0);
            let vol: f64 = t.quote_volume.parse().unwrap_or(0.0);
            let sign = if change >= 0.0 { "+" } else { "" };
            println!(
                "  {:<16} ${:<12.4} {:>7}  Vol: ${:.0}  [{}]",
                t.symbol, price, format!("{}{}%", sign, change), vol, t.source
            );
        }
    }

    if data.futures.data.is_empty() && data.spot.data.is_empty() {
        printer.info("No markets found matching filters");
    }

    Ok(())
}

async fn get_funding_rates(args: MarketFundingRatesArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;

    let label = args.symbol.as_deref().unwrap_or("all symbols");
    info!("Fetching funding rates for {}", label);

    let rates = client.get_funding_rates(args.symbol.as_deref()).await?;

    if rates.is_empty() {
        printer.info("No funding rate data found");
        return Ok(());
    }

    printer.info(&format!("Funding rates for {} ({} exchanges)", label, rates.len()));
    for r in &rates {
        let rate_pct = r.funding_rate * 100.0;
        let sign = if rate_pct >= 0.0 { "+" } else { "" };
        let oi_str = r.open_interest
            .map(|oi| format!("  OI: ${:.0}", oi))
            .unwrap_or_default();
        println!("  {:<16} {}{:.6}%{}", r.exchange, sign, rate_pct, oi_str);
    }

    Ok(())
}

async fn get_open_interest(args: MarketOpenInterestArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;

    let label = args.symbol.as_deref().unwrap_or("all markets");
    info!("Fetching open interest for {}", label);

    let items = client.get_open_interest(args.symbol.as_deref()).await?;

    if items.is_empty() {
        printer.info("No open interest data found");
        return Ok(());
    }

    printer.info(&format!("Open interest — {} markets", items.len()));
    for item in &items {
        println!(
            "  {:<16} OI: ${:<16.0}  Price: ${:.4}  Vol 24h: ${:.0}",
            item.symbol, item.open_interest_usd, item.price, item.volume_usd
        );
    }

    Ok(())
}

async fn get_scanner(args: MarketScannerArgs, settings: &AppConfig, _format: OutputFormat) -> Result<()> {
    let client = Client::new(settings)?;

    info!("Scanning {} on {}", args.symbol, args.timeframe);

    let result = client.get_scanner(
        &args.symbol,
        Some(&args.timeframe),
        args.bars,
        args.swing_strength,
    ).await?;

    let sig = &result.signal;

    println!();
    println!("  {} / {} — {} {}  (strength {}/100)", result.symbol, args.timeframe, sig.direction, sig.confidence, sig.strength as u32);
    println!("  Entry:     ${:.4}", sig.entry);
    println!("  Stop Loss: ${:.4}  ({:.2}% risk)",
        sig.stop_loss,
        ((sig.stop_loss - sig.entry) / sig.entry * 100.0).abs()
    );
    println!("  TP1:       ${:.4}  ({:+.2}%)", sig.take_profit1, (sig.take_profit1 - sig.entry) / sig.entry * 100.0);
    println!("  TP2:       ${:.4}  ({:+.2}%)", sig.take_profit2, (sig.take_profit2 - sig.entry) / sig.entry * 100.0);
    println!("  TP3:       ${:.4}  ({:+.2}%)", sig.take_profit3, (sig.take_profit3 - sig.entry) / sig.entry * 100.0);
    println!("  R/R:       {:.2}x", sig.risk_reward_ratio);
    println!("  Note:      {}", sig.reasoning);
    println!();
    Ok(())
}

async fn get_volume_snapshot(_args: MarketVolumeSnapshotArgs, settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;

    info!("Fetching volume snapshot");

    let exchanges = client.get_volume_snapshot().await?;

    if exchanges.is_empty() {
        printer.info("No volume snapshot data found");
        return Ok(());
    }

    printer.info(&format!("Volume snapshot — {} exchanges", exchanges.len()));
    for ex in &exchanges {
        println!(
            "  {:<20} Vol 24h: ${:<16.0}  OI: ${:<16.0}  TVL: ${:.0}  [{}]",
            ex.display_name, ex.total_volume_24h, ex.total_open_interest, ex.tvl, ex.chain
        );
    }

    Ok(())
}
