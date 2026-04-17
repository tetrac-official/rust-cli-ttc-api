//! Morning market brief — single-command snapshot of everything that matters at session open.
//!
//! Runs all fetches concurrently:
//!   - Watchlist futures tickers (prices, 24h%, volume, OI, funding)
//!   - Scanner signals for each watchlist symbol (parallel)
//!   - Portfolio: balance + positions
//!   - Open orders
//!
//! All sections degrade gracefully to "—" if the session is unavailable.

use crate::api::Client;
use crate::cli::BriefArgs;
use crate::commands::common::get_credentials;
use crate::config::AppConfig;
use crate::error::Result;
use crate::models::{Balance, HybridTicker, Order, Position, ScannerResult};
use colored::Colorize;
use std::time::{SystemTime, UNIX_EPOCH};

const TOKEN_EXPIRY_SECS: u64 = 24 * 3600;

// ── Entry point ────────────────────────────────────────────────────────────

pub async fn execute(args: BriefArgs, settings: &AppConfig) -> Result<()> {
    let symbols: Vec<String> = match &args.watchlist {
        Some(wl) => wl.iter().map(|s| s.to_uppercase()).collect(),
        None => settings
            .watchlist
            .symbols
            .iter()
            .map(|s| s.to_uppercase())
            .collect(),
    };

    // Header
    let now = chrono::Local::now();
    let date_str = now.format("%a %d %b %Y  %H:%M").to_string();
    let w = 64usize;
    println!();
    println!("{}", "━".repeat(w));
    println!(
        "  MORNING BRIEF — {}  │  {}",
        args.exchange.to_uppercase(),
        date_str
    );
    println!("{}", "━".repeat(w));

    // Session check (local, instant)
    let (session_ok, session_msg) = check_session(settings);
    println!();
    if session_ok {
        println!(
            "  {}  Session: {}",
            "✓".green().bold(),
            session_msg.dimmed()
        );
    } else {
        println!(
            "  {}  Session: {}  — run: skill-trading login",
            "✗".red().bold(),
            session_msg.red()
        );
    }

    // Build auth client — Option<Client> (None if session missing)
    let auth_client: Option<Client> = Client::new(settings).ok();
    let credentials = get_credentials(&args.exchange, None, None, None, settings).ok();

    // Spawn per-symbol scanner tasks concurrently (preserves symbol order)
    let scanner_handles: Vec<_> = symbols
        .iter()
        .map(|sym| {
            let client = auth_client.clone();
            let sym = sym.clone();
            let tf = args.timeframe.clone();
            tokio::spawn(async move {
                match client {
                    Some(c) => c
                        .get_scanner(&sym, Some(&tf), Some(1000), Some(10))
                        .await
                        .ok(),
                    None => None,
                }
            })
        })
        .collect();

    // Parallel fetch: tickers + portfolio + orders (all need auth client)
    let (raw_tickers, portfolio_data, orders_data) = tokio::join!(
        fetch_tickers(auth_client.as_ref()),
        fetch_portfolio(auth_client.as_ref(), credentials.clone(), &args.exchange),
        fetch_orders(auth_client.as_ref(), credentials.clone(), &args.exchange),
    );

    // Collect scanner results in symbol order
    let mut scanner_results: Vec<Option<ScannerResult>> = Vec::new();
    for handle in scanner_handles {
        let r = match handle.await {
            Ok(Some(sr)) => Some(sr),
            _ => None,
        };
        scanner_results.push(r);
    }

    // Render
    println!();
    print_prices(&symbols, &raw_tickers);
    print_signals(&symbols, &scanner_results, &args.timeframe);
    let portfolio_health = print_portfolio(&portfolio_data, settings);
    print_orders(&orders_data);

    // Overall status banner
    let worst = if !session_ok {
        HealthLevel::Watch
    } else {
        HealthLevel::Healthy
    };
    let overall = worst.max(portfolio_health);
    println!("{}", "━".repeat(w));
    match overall {
        HealthLevel::Healthy => {
            println!("  OVERALL: {}", "READY — no issues detected".green().bold())
        }
        HealthLevel::Watch => println!(
            "  OVERALL: {} — review warnings above before trading",
            "WATCH".yellow().bold()
        ),
        HealthLevel::Danger => println!(
            "  OVERALL: {} — address risk before opening new positions",
            "DANGER".red().bold()
        ),
    }
    println!("{}", "━".repeat(w));
    println!();

    Ok(())
}

// ── Section printers ───────────────────────────────────────────────────────

fn print_prices(symbols: &[String], tickers: &[ParsedTicker]) {
    println!(
        "  {} WATCHLIST PRICES {}",
        "──".dimmed(),
        "─".repeat(43).dimmed()
    );
    println!(
        "  {:<14} {:>12}  {:>8}  {:>10}  {:>10}  {:>9}",
        "Symbol", "Price", "24h%", "Volume", "OI", "Funding"
    );
    println!("  {}", "─".repeat(66));

    for sym in symbols {
        let upper = sym.to_uppercase();
        match tickers.iter().find(|t| t.symbol.to_uppercase() == upper) {
            None => println!("  {:<14}  {}", upper, "—".dimmed()),
            Some(t) => {
                let pct_s = format!("{:+.2}%", t.change_pct);
                let pct_colored = if t.change_pct >= 0.0 {
                    pct_s.green().to_string()
                } else {
                    pct_s.red().to_string()
                };
                let funding_s = if t.funding != 0.0 {
                    let s = format!("{:+.4}%", t.funding * 100.0);
                    if t.funding >= 0.0 {
                        s.green().to_string()
                    } else {
                        s.red().to_string()
                    }
                } else {
                    "—".dimmed().to_string()
                };
                println!(
                    "  {:<14} {:>12}  {:>8}  {:>10}  {:>10}  {:>9}",
                    upper,
                    fmt_price(t.price),
                    pct_colored,
                    fmt_volume(t.volume),
                    if t.oi > 0.0 {
                        fmt_volume(t.oi)
                    } else {
                        "—".to_string()
                    },
                    funding_s,
                );
            }
        }
    }
    println!();
}

fn print_signals(symbols: &[String], results: &[Option<ScannerResult>], timeframe: &str) {
    let pad = 47usize.saturating_sub(timeframe.len());
    println!(
        "  {} SIGNALS ({}) {}",
        "──".dimmed(),
        timeframe,
        "─".repeat(pad).dimmed()
    );

    for (sym, result) in symbols.iter().zip(results.iter()) {
        match result {
            None => println!("  {:<14}  {}", sym, "—".dimmed()),
            Some(r) => {
                let sig = &r.signal;
                let dir = sig.direction.to_uppercase();
                let dir_colored = match dir.as_str() {
                    "LONG" => dir.green().bold().to_string(),
                    "SHORT" => dir.red().bold().to_string(),
                    _ => dir.dimmed().to_string(),
                };

                if dir == "NEUTRAL" {
                    println!("  {:<14}  {}  —", sym, dir_colored);
                } else {
                    let sl_s = sig
                        .stop_loss
                        .map(|sl| format!("  SL {}", fmt_price(sl)))
                        .unwrap_or_default();
                    let tp_s = sig
                        .take_profit1
                        .map(|tp| format!("  TP1 {}", fmt_price(tp)))
                        .unwrap_or_default();
                    let rr_s = sig
                        .risk_reward_ratio
                        .map(|rr| format!("  R/R {:.1}x", rr))
                        .unwrap_or_default();
                    println!(
                        "  {:<14}  {}  {:4}  {:3}    {}{}{}{}",
                        sym,
                        dir_colored,
                        sig.confidence.to_uppercase(),
                        sig.strength as u32,
                        fmt_price(sig.entry),
                        sl_s,
                        tp_s,
                        rr_s,
                    );
                }
            }
        }
    }
    println!();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HealthLevel {
    Healthy,
    Watch,
    Danger,
}

fn print_portfolio(
    data: &Option<(Vec<Balance>, Vec<Position>)>,
    settings: &AppConfig,
) -> HealthLevel {
    println!("  {} PORTFOLIO {}", "──".dimmed(), "─".repeat(51).dimmed());

    let Some((balances, positions)) = data else {
        println!("  {}", "— session required —".dimmed());
        println!();
        return HealthLevel::Healthy;
    };

    let cfg = &settings.portfolio;
    let bal = balances.iter().find(|b| b.asset.to_uppercase() == "USDT");
    let total = bal.map(|b| b.balance).unwrap_or(0.0);
    let avail = bal.map(|b| b.available).unwrap_or(0.0);
    let locked = bal.and_then(|b| b.locked).unwrap_or(0.0);
    let util = if total > 0.0 {
        locked / total * 100.0
    } else {
        0.0
    };

    let mut health = HealthLevel::Healthy;

    let util_s = if util > cfg.max_margin_utilization {
        health = health.max(HealthLevel::Watch);
        format!("{:.1}% {}", util, "[WATCH]".yellow())
    } else {
        format!("{:.1}%", util)
    };

    let total_s = format!("${:.2}", total).bold();
    println!(
        "  Balance: {total_s}  Available: ${avail:.2}  Locked: ${locked:.2}  Utilization: {util_s}",
        avail = avail,
        locked = locked,
    );

    if positions.is_empty() {
        println!("  Positions: {}", "none open".dimmed());
    } else {
        println!();
        println!(
            "  {:<14} {:>5}  {:>3}   {:>9}   {:<22}  Liq dist",
            "Symbol", "Dir", "Lev", "Notional", "PnL"
        );
        println!("  {}", "─".repeat(70));

        for pos in positions {
            let pnl = pos.effective_pnl();
            let notional = pos.notional.unwrap_or(0.0);
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
            let liq_dist = if liq_price > 0.0 && pos.mark_price > 0.0 {
                ((pos.mark_price - liq_price) / pos.mark_price * 100.0).abs()
            } else {
                100.0
            };

            if liq_dist < cfg.min_liq_distance_pct {
                health = health.max(HealthLevel::Danger);
            }
            if notional > cfg.max_position_notional {
                health = health.max(HealthLevel::Watch);
            }

            let pnl_s = format!("{:+.2} ({:+.2}%)", pnl, pnl_pct);
            let pnl_colored = if pnl >= 0.0 {
                pnl_s.green().to_string()
            } else {
                pnl_s.red().to_string()
            };
            let liq_s = if liq_dist < cfg.min_liq_distance_pct {
                format!("{:.2}% {}", liq_dist, "[DANGER]".red())
            } else if liq_dist < cfg.min_liq_distance_pct * 2.0 {
                format!("{:.2}% {}", liq_dist, "[WATCH]".yellow())
            } else {
                format!("{:.2}%", liq_dist)
            };

            println!(
                "  {:<14} {:>5}  {:>2}x   {:>9}   {:<22}  {}",
                pos.symbol,
                pos.side.to_uppercase(),
                pos.leverage,
                format!("${:.0}", notional),
                pnl_colored,
                liq_s,
            );
        }
    }
    println!();
    health
}

fn print_orders(data: &Option<Vec<Order>>) {
    println!(
        "  {} OPEN ORDERS {}",
        "──".dimmed(),
        "─".repeat(49).dimmed()
    );
    match data {
        None => println!("  {}", "— session required —".dimmed()),
        Some(orders) if orders.is_empty() => println!("  {}", "none".dimmed()),
        Some(orders) => {
            println!(
                "  {:<14} {:>5}  {:>10}  {:>12}  {:>8}",
                "Symbol", "Side", "Type", "Price", "Qty"
            );
            println!("  {}", "─".repeat(55));
            for o in orders {
                let price_s = if o.price > 0.0 {
                    fmt_price(o.price)
                } else {
                    "market".to_string()
                };
                let side_colored = match o.side.to_lowercase().as_str() {
                    "buy" => o.side.to_uppercase().green().to_string(),
                    "sell" => o.side.to_uppercase().red().to_string(),
                    _ => o.side.to_uppercase(),
                };
                println!(
                    "  {:<14} {:>5}  {:>10}  {:>12}  {:>8}",
                    o.symbol,
                    side_colored,
                    o.order_type.to_uppercase(),
                    price_s,
                    o.quantity
                );
            }
        }
    }
    println!();
}

// ── Data fetchers ──────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct ParsedTicker {
    symbol: String,
    price: f64,
    change_pct: f64,
    volume: f64,
    oi: f64,
    funding: f64,
}

impl From<HybridTicker> for ParsedTicker {
    fn from(t: HybridTicker) -> Self {
        Self {
            symbol: t.symbol,
            price: t.last_price.parse().unwrap_or(0.0),
            change_pct: t.price_change_percent.parse().unwrap_or(0.0),
            volume: t
                .quote_volume
                .parse()
                .unwrap_or_else(|_| t.volume.parse().unwrap_or(0.0)),
            oi: t
                .open_interest
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            funding: t
                .funding
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
        }
    }
}

async fn fetch_tickers(client: Option<&Client>) -> Vec<ParsedTicker> {
    let Some(c) = client else {
        return vec![];
    };
    match c
        .get_hybrid_tickers(Some("futures"), None, None, None, None, None, None, None)
        .await
    {
        Ok(data) => data
            .futures
            .data
            .into_iter()
            .map(ParsedTicker::from)
            .collect(),
        Err(_) => vec![],
    }
}

async fn fetch_portfolio(
    client: Option<&Client>,
    creds: Option<crate::models::ExchangeCredentials>,
    exchange: &str,
) -> Option<(Vec<Balance>, Vec<Position>)> {
    let client = client?;
    let creds = creds?;
    tokio::try_join!(
        client.get_balance(exchange, creds.clone()),
        client.get_positions(exchange, None, creds),
    )
    .ok()
}

async fn fetch_orders(
    client: Option<&Client>,
    creds: Option<crate::models::ExchangeCredentials>,
    exchange: &str,
) -> Option<Vec<Order>> {
    let client = client?;
    let creds = creds?;
    client.get_orders(exchange, None, creds).await.ok()
}

// ── Session check ─────────────────────────────────────────────────────────

fn check_session(settings: &AppConfig) -> (bool, String) {
    if !settings
        .api_key
        .as_ref()
        .map(|k| !k.is_empty())
        .unwrap_or(false)
    {
        return (false, "TTC_AUTH_TOKEN not set".to_string());
    }
    match std::env::var("TTC_TOKEN_ISSUED_AT")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
    {
        Some(issued_at) => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let elapsed = now.saturating_sub(issued_at);
            if elapsed >= TOKEN_EXPIRY_SECS {
                (false, format!("EXPIRED ({}h ago)", elapsed / 3600))
            } else {
                let rem = TOKEN_EXPIRY_SECS - elapsed;
                (
                    true,
                    format!("valid  {}h {}m remaining", rem / 3600, (rem % 3600) / 60),
                )
            }
        }
        None => (true, "valid (issued-at not recorded)".to_string()),
    }
}

// ── Format helpers ─────────────────────────────────────────────────────────

fn fmt_price(p: f64) -> String {
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

fn fmt_volume(v: f64) -> String {
    if v >= 1_000_000_000.0 {
        format!("${:.2}B", v / 1_000_000_000.0)
    } else if v >= 1_000_000.0 {
        format!("${:.1}M", v / 1_000_000.0)
    } else if v >= 1_000.0 {
        format!("${:.1}K", v / 1_000.0)
    } else {
        format!("${:.2}", v)
    }
}
