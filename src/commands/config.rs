//! Config command implementations

use crate::cli::*;
use crate::config::{AppConfig, ExchangeCredentialConfig};
use crate::error::Result;
use crate::output::{OutputFormat, Printer};

pub async fn execute(
    cmd: ConfigCommands,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    match cmd.command {
        ConfigSubcommands::Init => init_config(format).await,
        ConfigSubcommands::Show => show_config(settings, format).await,
        ConfigSubcommands::SetDefault(args) => set_default(args, format).await,
        ConfigSubcommands::AddExchange(args) => add_exchange(args, format).await,
        ConfigSubcommands::RmExchange(args) => remove_exchange(args, settings, format).await,
        ConfigSubcommands::Path => show_path(format).await,
    }
}

async fn init_config(format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);

    let config_path = AppConfig::default_config_path();

    if config_path.exists() {
        printer.warning(&format!(
            "Config file already exists at: {}",
            config_path.display()
        ));
        printer.info("Use 'config show' to view current config");
        return Ok(());
    }

    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::error::TtcError::Config(format!("Failed to create config directory: {}", e))
            })?;
        }
    }

    let default_settings = AppConfig::default();
    let contents = toml::to_string_pretty(&default_settings).map_err(|e| {
        crate::error::TtcError::Config(format!("Failed to serialize config: {}", e))
    })?;

    std::fs::write(&config_path, contents)
        .map_err(|e| crate::error::TtcError::Config(format!("Failed to write config: {}", e)))?;

    printer.success("Config file created!");
    println!();
    println!("  Location: {}", config_path.display());
    println!();
    println!("  Edit the config file to add your exchange credentials:");
    println!("   [exchanges.phemex]");
    println!("   api_key = \"YOUR_API_KEY\"");
    println!("   api_secret = \"YOUR_API_SECRET\"");
    println!();
    println!("  Or set environment variables:");
    println!("   export TTC_AUTH_TOKEN=\"your-ttc-auth-token\"");
    println!("   export TTC_PUBLIC_KEY=\"your-ttc-public-key\"");
    println!();

    Ok(())
}

async fn show_config(settings: &AppConfig, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);

    printer.info("Current Configuration");
    println!();

    println!("  API Settings:");
    println!("   Base URL: {}", settings.api.base_url);
    println!("   Timeout: {}s", settings.api.timeout);
    println!("   Max Retries: {}", settings.api.max_retries);
    println!("   Retry Delay: {}ms", settings.api.retry_delay_ms);
    println!();

    println!("  Output Settings:");
    println!("   Format: {}", settings.output.format);
    println!("   Color: {}", settings.output.color);
    println!();

    println!("  Trading Settings:");
    println!(
        "   Default Exchange: {}",
        settings.exchange.as_deref().unwrap_or("none")
    );
    println!(
        "   Default Leverage: {}x",
        settings.trading.default_leverage
    );
    println!("   Confirm Orders: {}", settings.trading.confirm_orders);
    println!("   Dry Run: {}", settings.trading.dry_run);
    println!();

    println!("  Configured Exchanges:");
    if settings.exchanges.is_empty() {
        println!("   (none)");
    } else {
        for (name, creds) in &settings.exchanges {
            let masked_key = mask_key(&creds.api_key);
            println!("   {} - API Key: {}...", name, masked_key);
        }
    }
    println!();

    Ok(())
}

async fn set_default(args: ConfigSetDefaultArgs, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let config_path = AppConfig::discover_config_path();

    let mut config = AppConfig::load_from_file(&Some(config_path.clone())).unwrap_or_default();

    config.exchange = Some(args.exchange.clone());

    let contents = toml::to_string_pretty(&config).map_err(|e| {
        crate::error::TtcError::Config(format!("Failed to serialize config: {}", e))
    })?;

    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::error::TtcError::Config(format!("Failed to create config dir: {}", e))
            })?;
        }
    }

    std::fs::write(&config_path, &contents)
        .map_err(|e| crate::error::TtcError::Config(format!("Failed to write config: {}", e)))?;

    printer.success(&format!("Default exchange set to: {}", args.exchange));
    printer.info(&format!("Saved to: {}", config_path.display()));

    Ok(())
}

async fn add_exchange(args: ConfigAddExchangeArgs, format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);

    let _creds = ExchangeCredentialConfig {
        api_key: args.api_key.clone(),
        api_secret: args.api_secret.clone(),
        passphrase: args.passphrase.clone(),
    };

    printer.success(&format!(
        "Exchange '{}' would be added to config",
        args.exchange
    ));
    printer.info("Note: Configuration changes require manual editing of the config file or environment variables.");
    printer.warning(
        "Credentials stored locally. Consider using environment variables for production.",
    );

    Ok(())
}

async fn remove_exchange(
    args: ConfigRmExchangeArgs,
    settings: &AppConfig,
    format: OutputFormat,
) -> Result<()> {
    let printer = Printer::new(format);

    if settings.exchanges.contains_key(&args.exchange) {
        printer.success(&format!(
            "Exchange '{}' would be removed from config",
            args.exchange
        ));
        printer.info("Note: Configuration changes require manual editing of the config file.");
    } else {
        printer.warning(&format!("Exchange '{}' not found in config", args.exchange));
    }

    Ok(())
}

async fn show_path(format: OutputFormat) -> Result<()> {
    let printer = Printer::new(format);
    let config_path = AppConfig::default_config_path();

    printer.info("Config file location:");
    println!();
    println!("   {}", config_path.display());
    println!();

    if config_path.exists() {
        printer.success("Config file exists");
    } else {
        printer.warning("Config file does not exist. Run 'config init' to create it.");
    }

    Ok(())
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    format!("{}****", &key[..4])
}
