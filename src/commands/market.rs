//! Market data command implementations

use crate::api::Client;
use crate::cli::*;
use crate::commands::common::get_credentials;
use crate::config::AppConfig;
use crate::error::{Result, TtcError};
use crate::models::*;
use crate::output::{OutputFormat, Printer};
use colored::Colorize;
use tracing::info;

pub async fn execute(
    cmd: MarketCommands,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    match cmd.command {
        MarketSubcommands::Tickers(args) => get_tickers(args, settings, format).await,
        MarketSubcommands::BestBidAsk(args) => get_best_bid_ask(args, settings, format).await,
        MarketSubcommands::HybridTickers(args) => get_hybrid_tickers(args, settings, format).await,
        MarketSubcommands::FundingRates(args) => get_funding_rates(args, settings, format).await,
        MarketSubcommands::OpenInterest(args) => get_open_interest(args, settings, format).await,
        MarketSubcommands::VolumeSnapshot(args) => {
            get_volume_snapshot(args, settings, format).await
        }
        MarketSubcommands::Scanner(args) => get_scanner(args, settings, format).await,
        MarketSubcommands::Alert(args) => run_alert(args, settings).await,
    }
}

async fn get_tickers(
    args: MarketTickersArgs,
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

    info!("Fetching tickers from {}", args.exchange);

    let params = GetTickersParams {
        symbol: args.symbol.clone(),
    };

    let tickers = client
        .get_tickers(&args.exchange, params, credentials)
        .await?;

    if tickers.is_empty() {
        printer.info(&format!("No tickers found on {}", args.exchange));
    } else {
        printer.info(&format!(
            "Found {} ticker(s) on {}",
            tickers.len(),
            args.exchange
        ));
    }

    printer.print_list(&tickers);

    Ok(())
}

async fn get_best_bid_ask(
    args: MarketBestBidAskArgs,
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

    info!(
        "Fetching best bid/ask for {} on {}",
        args.symbol, args.exchange
    );

    let params = GetBestBidAskParams {
        symbol: args.symbol.clone(),
    };

    let result = client
        .get_best_bid_ask(&args.exchange, params, credentials)
        .await?;

    let bid: f64 = result.best_bid.price.parse().unwrap_or(0.0);
    let ask: f64 = result.best_ask.price.parse().unwrap_or(0.0);
    let spread = ask - bid;

    printer.success(&format!(
        "Best Bid/Ask for {} on {}",
        args.symbol, args.exchange
    ));
    println!("  Bid: {} @ ${:.4}", result.best_bid.quantity, bid);
    println!("  Ask: {} @ ${:.4}", result.best_ask.quantity, ask);
    println!(
        "  Spread: ${:.4} ({:.4}%)",
        spread,
        if bid > 0.0 {
            (spread / bid) * 100.0
        } else {
            0.0
        }
    );

    Ok(())
}

async fn get_hybrid_tickers(
    args: MarketHybridTickersArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;

    let market_type = args.market_type.map(|t| match t {
        MarketTypeArg::Spot => "spot",
        MarketTypeArg::Futures => "futures",
    });

    info!("Fetching hybrid tickers");

    let data = client
        .get_hybrid_tickers(
            market_type,
            args.source.as_deref(),
            args.symbol.as_deref(),
            args.min_volume,
            args.min_price,
            args.max_price,
            args.up,
            args.down,
        )
        .await?;

    let show_spot = market_type.map(|t| t == "spot").unwrap_or(true);
    let show_futures = market_type.map(|t| t == "futures").unwrap_or(true);

    if show_futures && !data.futures.data.is_empty() {
        printer.info(&format!("Futures: {} markets", data.futures.data.len()));
        for t in &data.futures.data {
            let price: f64 = t.last_price.parse().unwrap_or(0.0);
            let change: f64 = t.price_change_percent.parse().unwrap_or(0.0);
            let vol: f64 = t.quote_volume.parse().unwrap_or(0.0);
            let oi: f64 = t
                .open_interest
                .as_deref()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0.0);
            let funding: f64 = t.funding.as_deref().unwrap_or("0").parse().unwrap_or(0.0);
            let sign = if change >= 0.0 { "+" } else { "" };
            println!(
                "  {:<16} ${:<12.4} {:>7}  Vol: ${:.0}  OI: ${:.0}  Fund: {:.4}%  [{}]",
                t.symbol,
                price,
                format!("{}{}%", sign, change),
                vol,
                oi,
                funding * 100.0,
                t.source
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
                t.symbol,
                price,
                format!("{}{}%", sign, change),
                vol,
                t.source
            );
        }
    }

    if data.futures.data.is_empty() && data.spot.data.is_empty() {
        printer.info("No markets found matching filters");
    }

    Ok(())
}

async fn get_funding_rates(
    args: MarketFundingRatesArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    let printer = Printer::new(format);
    let client = Client::new(settings)?;

    let label = args.symbol.as_deref().unwrap_or("all symbols");
    info!("Fetching funding rates for {}", label);

    let rates = client.get_funding_rates(args.symbol.as_deref()).await?;

    if rates.is_empty() {
        printer.info("No funding rate data found");
        return Ok(());
    }

    printer.info(&format!(
        "Funding rates for {} ({} exchanges)",
        label,
        rates.len()
    ));
    for r in &rates {
        let rate_pct = r.funding_rate * 100.0;
        let sign = if rate_pct >= 0.0 { "+" } else { "" };
        let oi_str = r
            .open_interest
            .map(|oi| format!("  OI: ${:.0}", oi))
            .unwrap_or_default();
        println!("  {:<16} {}{:.6}%{}", r.exchange, sign, rate_pct, oi_str);
    }

    Ok(())
}

async fn get_open_interest(
    args: MarketOpenInterestArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
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

async fn get_scanner(
    args: MarketScannerArgs,
    settings: &AppConfig,
    _format: OutputFormat,
) -> Result<()> {
    let client = Client::new(settings)?;

    // ── Single-symbol mode ───────────────────────────────────────────────────
    if let Some(ref sym) = args.symbol {
        info!("Scanning {} on {}", sym, args.timeframe);
        let result = client
            .get_scanner(sym, Some(&args.timeframe), args.bars, args.swing_strength)
            .await?;
        print_single_scan(&result, &args.timeframe);
        return Ok(());
    }

    // ── Watchlist mode ───────────────────────────────────────────────────────
    let symbols: Vec<String> = match &args.watchlist {
        Some(wl) => wl.iter().map(|s| s.to_uppercase()).collect(),
        None => settings
            .watchlist
            .symbols
            .iter()
            .map(|s| s.to_uppercase())
            .collect(),
    };

    if symbols.is_empty() {
        return Err(TtcError::InvalidOrder(
            "No symbols to scan. Use --symbol BTCUSDT, --watchlist BTC,ETH, or set [watchlist] in config.toml".to_string()
        ));
    }

    info!("Scanning {} symbols on {}", symbols.len(), args.timeframe);

    // Spawn all scans in parallel, preserve order
    let handles: Vec<_> = symbols
        .iter()
        .map(|sym| {
            let c = client.clone();
            let sym = sym.clone();
            let tf = args.timeframe.clone();
            let bars = args.bars;
            let swing = args.swing_strength;
            tokio::spawn(async move { c.get_scanner(&sym, Some(&tf), bars, swing).await })
        })
        .collect();

    let mut results: Vec<(String, Option<ScannerResult>)> = Vec::new();
    for (sym, handle) in symbols.iter().zip(handles) {
        let r = handle.await.ok().and_then(|r| r.ok());
        results.push((sym.clone(), r));
    }

    // Apply filters
    let conf_filter = if args.only_high { "HIGH" } else { "" };
    let min_rr = args.min_rr;

    let mut matched = 0usize;
    let mut neutral = 0usize;
    let mut skipped = 0usize; // signals that exist but didn't pass the filter

    // Header
    let filter_desc = {
        let mut parts = vec![];
        if !conf_filter.is_empty() {
            parts.push(format!("{} only", conf_filter));
        }
        parts.push(format!("R/R ≥ {:.1}", min_rr));
        parts.join("  ")
    };
    println!();
    println!(
        "  Scanner — {}  │  {} symbols  │  filter: {}",
        args.timeframe,
        symbols.len(),
        filter_desc
    );
    println!("  {}", "─".repeat(88));
    println!(
        "  {:<14}  {:<7}  {:<6}  {:>3}  {:>12}  {:>12}  {:>12}  {:>6}",
        "Symbol", "Dir", "Conf", "Str", "Entry", "SL", "TP1", "R/R"
    );
    println!("  {}", "─".repeat(88));

    for (sym, result) in &results {
        match result {
            None => {
                println!("  {:<14}  {}", sym, "—  (fetch error)".dimmed());
            }
            Some(r) => {
                let sig = &r.signal;
                let dir = sig.direction.to_uppercase();

                if dir == "NEUTRAL" {
                    neutral += 1;
                    println!("  {:<14}  {}", sym, "NEUTRAL  —".dimmed());
                    continue;
                }

                let rr = sig.risk_reward_ratio.unwrap_or(0.0);
                let passes_rr = rr >= min_rr || min_rr == 0.0;
                let passes_conf =
                    conf_filter.is_empty() || sig.confidence.to_uppercase() == conf_filter;

                if !passes_rr || !passes_conf {
                    skipped += 1;
                    let rr_s = if rr > 0.0 {
                        format!("{:.1}x", rr)
                    } else {
                        "—".to_string()
                    };
                    println!(
                        "  {:<14}  {}  {}  {:>3}  {:>12}  {:>12}  {:>12}  {:>6}  {}",
                        sym,
                        dir.dimmed(),
                        sig.confidence.to_uppercase().dimmed(),
                        sig.strength as u32,
                        scan_fmt_price(sig.entry).dimmed().to_string(),
                        sig.stop_loss
                            .map(scan_fmt_price)
                            .unwrap_or_else(|| "—".to_string())
                            .dimmed()
                            .to_string(),
                        sig.take_profit1
                            .map(scan_fmt_price)
                            .unwrap_or_else(|| "—".to_string())
                            .dimmed()
                            .to_string(),
                        rr_s.dimmed().to_string(),
                        "filtered".dimmed(),
                    );
                    continue;
                }

                matched += 1;
                let dir_colored = match dir.as_str() {
                    "LONG" => dir.green().bold().to_string(),
                    "SHORT" => dir.red().bold().to_string(),
                    _ => dir.clone(),
                };
                let rr_s = if rr > 0.0 {
                    format!("{:.1}x", rr)
                } else {
                    "—".to_string()
                };
                let rr_colored = if rr >= 3.0 {
                    rr_s.green().to_string()
                } else {
                    rr_s
                };

                println!(
                    "  {:<14}  {:<7}  {:<6}  {:>3}  {:>12}  {:>12}  {:>12}  {:>6}",
                    sym.bold().to_string(),
                    dir_colored,
                    sig.confidence.to_uppercase(),
                    sig.strength as u32,
                    scan_fmt_price(sig.entry),
                    sig.stop_loss
                        .map(scan_fmt_price)
                        .unwrap_or_else(|| "—".to_string()),
                    sig.take_profit1
                        .map(scan_fmt_price)
                        .unwrap_or_else(|| "—".to_string()),
                    rr_colored,
                );
            }
        }
    }

    println!("  {}", "─".repeat(88));
    println!(
        "  {} signal(s) match  │  {} NEUTRAL  │  {} below filter",
        matched.to_string().bold(),
        neutral,
        skipped
    );
    println!();

    Ok(())
}

fn print_single_scan(result: &ScannerResult, timeframe: &str) {
    let sig = &result.signal;
    println!();
    println!(
        "  {} / {} — {} {}  (strength {}/100)",
        result.symbol,
        timeframe,
        sig.direction.to_uppercase(),
        sig.confidence.to_uppercase(),
        sig.strength as u32
    );
    println!("  Entry:     {}", scan_fmt_price(sig.entry));
    if let Some(scan) = result.scans.first() {
        let m = &scan.momentum;
        let sign = if m.rise_per_bar >= 0.0 { "+" } else { "" };
        println!(
            "  Gann unit: ${:.6}/bar (1x1)  |  Momentum: {}{:.6}/bar ({})  |  Avg range: ${:.6}/bar",
            scan.price_time_ratio, sign, m.rise_per_bar, m.trend_direction, m.avg_range
        );
    }
    if let Some(sl) = sig.stop_loss {
        println!(
            "  Stop Loss: {}  ({:.2}% risk)",
            scan_fmt_price(sl),
            ((sl - sig.entry) / sig.entry * 100.0).abs()
        );
    } else {
        println!("  Stop Loss: n/a");
    }
    if let Some(tp1) = sig.take_profit1 {
        println!(
            "  TP1:       {}  ({:+.2}%)",
            scan_fmt_price(tp1),
            (tp1 - sig.entry) / sig.entry * 100.0
        );
    }
    if let Some(tp2) = sig.take_profit2 {
        println!(
            "  TP2:       {}  ({:+.2}%)",
            scan_fmt_price(tp2),
            (tp2 - sig.entry) / sig.entry * 100.0
        );
    }
    if let Some(tp3) = sig.take_profit3 {
        println!(
            "  TP3:       {}  ({:+.2}%)",
            scan_fmt_price(tp3),
            (tp3 - sig.entry) / sig.entry * 100.0
        );
    }
    if let Some(rr) = sig.risk_reward_ratio {
        println!("  R/R:       {:.2}x", rr);
    }
    println!("  Note:      {}", sig.reasoning);
    println!();
}

fn scan_fmt_price(p: f64) -> String {
    if p >= 10_000.0 {
        format!("${:.0}", p)
    } else if p >= 100.0 {
        format!("${:.2}", p)
    } else if p >= 1.0 {
        format!("${:.4}", p)
    } else {
        format!("${:.6}", p)
    }
}

async fn get_volume_snapshot(
    _args: MarketVolumeSnapshotArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
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

// ── Price Level Alert ──────────────────────────────────────────────────────

async fn run_alert(args: MarketAlertArgs, settings: &AppConfig) -> Result<()> {
    if args.upper.is_none() && args.lower.is_none() {
        return Err(TtcError::InvalidOrder(
            "Specify at least one of --upper or --lower".to_string(),
        ));
    }

    let client = Client::new(settings)?;
    let sym = args.symbol.to_uppercase();

    println!();
    println!("  Price alert — {}", sym.bold());
    if let Some(u) = args.upper {
        println!(
            "  Upper: {}  (alert on breakout above)",
            alert_fmt_price(u).green()
        );
    }
    if let Some(l) = args.lower {
        println!(
            "  Lower: {}  (alert on breakdown below)",
            alert_fmt_price(l).red()
        );
    }
    println!(
        "  Interval: {}s  |  {}",
        args.interval,
        if args.continuous {
            "continuous (Ctrl+C to stop)"
        } else {
            "stop on first trigger"
        }
    );
    println!();

    let mut upper_fired = false;
    let mut lower_fired = false;

    loop {
        // Fetch current price via hybrid tickers (no exchange credentials needed)
        let price = match fetch_price(&client, &sym).await {
            Some(p) => p,
            None => {
                eprintln!("  [warn] Could not fetch price for {}", sym);
                tokio::time::sleep(tokio::time::Duration::from_secs(args.interval)).await;
                continue;
            }
        };

        let now = chrono::Local::now().format("%H:%M:%S").to_string();

        // Check upper level
        let mut fired_this_tick = false;
        if !upper_fired {
            if let Some(u) = args.upper {
                if price >= u {
                    println!();
                    println!(
                        "  {} [{}]  {} CROSSED ABOVE {}  (now {})",
                        "!! ALERT".red().bold(),
                        now,
                        sym.bold(),
                        alert_fmt_price(u).green().bold(),
                        alert_fmt_price(price).bold(),
                    );
                    println!();
                    upper_fired = true;
                    fired_this_tick = true;
                    maybe_notify(&sym, &format!("crossed ABOVE {}", alert_fmt_price(u)));
                    if !args.continuous {
                        break;
                    }
                }
            }
        }

        // Check lower level
        if !lower_fired {
            if let Some(l) = args.lower {
                if price <= l {
                    println!();
                    println!(
                        "  {} [{}]  {} CROSSED BELOW {}  (now {})",
                        "!! ALERT".red().bold(),
                        now,
                        sym.bold(),
                        alert_fmt_price(l).red().bold(),
                        alert_fmt_price(price).bold(),
                    );
                    println!();
                    lower_fired = true;
                    fired_this_tick = true;
                    maybe_notify(&sym, &format!("crossed BELOW {}", alert_fmt_price(l)));
                    if !args.continuous {
                        break;
                    }
                }
            }
        }

        // Status line (when no trigger this tick)
        if !fired_this_tick {
            let upper_s = args
                .upper
                .map(|u| {
                    let pct = (u - price) / price * 100.0;
                    format!("  Upper: {} ({:+.2}%)", alert_fmt_price(u), pct)
                })
                .unwrap_or_default();
            let lower_s = args
                .lower
                .map(|l| {
                    let pct = (l - price) / price * 100.0;
                    format!("  Lower: {} ({:+.2}%)", alert_fmt_price(l), pct)
                })
                .unwrap_or_default();
            println!(
                "  [{}]  {}  {}{}{}",
                now,
                sym,
                alert_fmt_price(price),
                upper_s,
                lower_s
            );
        }

        // All levels fired?
        let all_fired = args.upper.map(|_| upper_fired).unwrap_or(true)
            && args.lower.map(|_| lower_fired).unwrap_or(true);
        if all_fired {
            println!("  All levels fired. Done.");
            break;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(args.interval)).await;
    }

    Ok(())
}

/// Fetch last price for a symbol from hybrid futures tickers.
async fn fetch_price(client: &Client, symbol: &str) -> Option<f64> {
    let data = client
        .get_hybrid_tickers(
            Some("futures"),
            None,
            Some(symbol),
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .ok()?;

    // Try futures first, then spot
    let ticker = data
        .futures
        .data
        .iter()
        .chain(data.spot.data.iter())
        .find(|t| t.symbol.to_uppercase() == symbol.to_uppercase())?;

    ticker.last_price.parse().ok()
}

/// macOS system notification via osascript (silent if unavailable).
fn maybe_notify(symbol: &str, message: &str) {
    let script = format!(
        r#"display notification "{} {}" with title "skill-trading alert""#,
        symbol, message
    );
    let _ = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output();
}

fn alert_fmt_price(p: f64) -> String {
    if p >= 10_000.0 {
        format!("${:.0}", p)
    } else if p >= 100.0 {
        format!("${:.2}", p)
    } else if p >= 1.0 {
        format!("${:.4}", p)
    } else {
        format!("${:.6}", p)
    }
}
