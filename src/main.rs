//! skill-trading - Execute trading operations on TTC Box across 15+ exchanges
//!
//! Place orders, manage positions, set leverage, and control risk.

use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

mod api;
mod cli;
mod commands;
mod config;
mod crypto;
mod error;
mod models;
mod output;

use crate::config::AppConfig;
use crate::error::Result;
use crate::output::OutputFormat;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Main CLI structure
#[derive(Parser, Debug)]
#[command(
    name = "skill-trading",
    version = VERSION,
    about = "Execute trading operations on TTC Box across 15+ exchanges",
    long_about = "Execute trading operations on TTC Box across 15+ exchanges.\n\
                  Place orders, manage positions, set leverage, and control risk.\n\n\
                  Designed for Claude Code and similar AI products.",
    propagate_version = true,
    disable_help_subcommand = true,
    next_line_help = true
)]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, env = "TTC_CONFIG", global = true)]
    pub config: Option<PathBuf>,

    /// TTC Auth Token
    #[arg(long, env = "TTC_AUTH_TOKEN", global = true)]
    pub api_key: Option<String>,

    /// TTC Public Key
    #[arg(long, env = "TTC_PUBLIC_KEY", global = true)]
    pub public_key: Option<String>,

    /// Exchange API Key
    #[arg(long, env = "EXCHANGE_API_KEY", global = true)]
    pub exchange_api_key: Option<String>,

    /// Exchange API Secret
    #[arg(long, env = "EXCHANGE_API_SECRET", global = true)]
    pub exchange_api_secret: Option<String>,

    /// Exchange API Passphrase (required by OKX, KuCoin, Orderly, Bitget, BloFin)
    #[arg(long, env = "EXCHANGE_API_PASSPHRASE", global = true)]
    pub exchange_api_passphrase: Option<String>,

    /// Default exchange name
    #[arg(short, long, env = "TTC_EXCHANGE", global = true)]
    pub exchange: Option<String>,

    /// Output format: table, json, csv, quiet
    #[arg(long, env = "TTC_OUTPUT", global = true)]
    pub output_format: Option<OutputFormat>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Disable colored output
    #[arg(long, env = "NO_COLOR", global = true)]
    pub no_color: bool,

    /// Dry run - don't execute, just show what would happen
    #[arg(long, global = true)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: cli::Commands,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Auto-load .env from current directory (silently ignore if not found)
    let _ = dotenvy::dotenv();

    // Pre-load config so config.toml's `exchange` field acts as the default
    // for --exchange. Only sets TTC_EXCHANGE if not already in env.
    if std::env::var("TTC_EXCHANGE").is_err() {
        if let Ok(pre_config) = AppConfig::load_from_file(&None) {
            if let Some(exchange) = pre_config.exchange {
                std::env::set_var("TTC_EXCHANGE", exchange);
            }
        }
    }

    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .pretty()
        .init();

    // Build settings from CLI args
    let mut settings = AppConfig::load_from_file(&cli.config)?;

    // Apply CLI overrides
    if let Some(ref key) = cli.api_key {
        settings.api_key = Some(key.clone());
    }
    if let Some(ref key) = cli.public_key {
        settings.public_key = Some(key.clone());
    }
    if let Some(ref key) = cli.exchange_api_key {
        settings.exchange_api_key = Some(key.clone());
    }
    if let Some(ref secret) = cli.exchange_api_secret {
        settings.exchange_api_secret = Some(secret.clone());
    }
    if let Some(ref passphrase) = cli.exchange_api_passphrase {
        settings.exchange_api_passphrase = Some(passphrase.clone());
    }
    if let Some(ref exchange) = cli.exchange {
        settings.exchange = Some(exchange.clone());
    }
    if cli.dry_run {
        settings.trading.dry_run = true;
    }
    if cli.no_color {
        settings.output.color = false;
    }

    // Determine output format
    let format = cli.output_format.unwrap_or(OutputFormat::Table);

    // Auto-refresh TTC session token if ≥ 23 hours old (before any API call)
    // Skip for commands that don't need auth or that manage auth themselves
    let skip_refresh = matches!(
        cli.command,
        cli::Commands::Login(_) | cli::Commands::Register(_) | cli::Commands::Info | cli::Commands::Config(_)
    );
    if !skip_refresh {
        match commands::login::try_silent_refresh(&settings).await {
            Ok(true) => {
                eprintln!("[auto-refresh] TTC session token refreshed.");
                // Reload fresh token into settings for this invocation
                settings.api_key = std::env::var("TTC_AUTH_TOKEN").ok();
                if let Ok(pk) = std::env::var("TTC_PUBLIC_KEY") {
                    settings.public_key = Some(pk);
                }
            }
            Ok(false) => {} // Token still fresh or credentials unavailable
            Err(e) => {
                eprintln!("[auto-refresh] Warning: token refresh failed: {e}");
            }
        }
    }

    // Execute command
    let result = match cli.command {
        cli::Commands::Order(cmd) => commands::order::execute(cmd, &settings, format).await,
        cli::Commands::Position(cmd) => commands::position::execute(cmd, &settings, format).await,
        cli::Commands::Account(cmd) => commands::account::execute(cmd, &settings, format).await,
        cli::Commands::Risk(cmd) => commands::risk::execute(cmd, &settings, format).await,
        cli::Commands::Config(cmd) => commands::config::execute(cmd, &settings, format).await,
        cli::Commands::Orders(cmd) => commands::orders::execute(cmd, &settings, format).await,
        cli::Commands::Market(cmd) => commands::market::execute(cmd, &settings, format).await,
        cli::Commands::Info => {
            println!("skill-trading v{}", VERSION);
            println!("Execute trading operations on TTC Box across 15+ exchanges.");
            Ok(())
        }
        cli::Commands::Login(cmd) => commands::login::execute(cmd, &settings).await,
        cli::Commands::Register(cmd) => commands::register::execute(cmd, &settings).await,
        cli::Commands::Twap(cmd) => commands::twap::execute(cmd, &settings).await,
        cli::Commands::TwapSlice(cmd) => commands::twap_slice::execute(cmd, &settings).await,
        cli::Commands::Portfolio(cmd) => commands::portfolio::execute(cmd, &settings, format).await,
        cli::Commands::Status => commands::status::execute(&settings).await,
        cli::Commands::Brief(cmd) => commands::brief::execute(cmd, &settings).await,
        cli::Commands::MarketMaker(cmd) => commands::market_maker::execute(cmd, &settings).await,
    };

    // Handle errors
    if let Err(ref e) = result {
        eprintln!("{} {}", "Error:".red().bold(), e);
        // Hint on auth failures: token may have expired between refresh check and API call
        if let crate::error::TtcError::Api { code: 401, .. } = e {
            eprintln!("Hint: session token expired. Run `skill-trading login` to refresh.");
        }
        std::process::exit(1);
    }

    Ok(())
}
